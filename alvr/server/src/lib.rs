mod bitrate;
mod buttons;
mod connection;
mod face_tracking;
mod haptics;
mod logging_backend;
mod openvr_props;
mod sockets;
mod statistics;
mod tracking;
mod web_server;

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    non_snake_case,
    clippy::unseparated_literal_suffix
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use alvr_common::{
    glam::Quat,
    log,
    once_cell::sync::{Lazy, OnceCell},
    parking_lot::{Mutex, RwLock},
    prelude::*,
};
use alvr_events::EventType;
use alvr_filesystem::{self as afs, Layout};
use alvr_packets::{
    ClientListAction, DecoderInitializationConfig, Haptics, ServerControlPacket, VideoPacketHeader,
};
use alvr_server_io::ServerDataManager;
use alvr_session::CodecType;
use bitrate::BitrateManager;
use connection::SHOULD_CONNECT_TO_CLIENTS;
use statistics::StatisticsManager;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr, CString},
    fs::File,
    io::Write,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Once,
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, RefreshKind, SystemExt};
use tokio::{
    runtime::Runtime,
    sync::{
        broadcast,
        mpsc::{self, error::TrySendError},
        Notify,
    },
};

static FILESYSTEM_LAYOUT: Lazy<Layout> = Lazy::new(|| {
    afs::filesystem_layout_from_openvr_driver_root_dir(&alvr_server_io::get_driver_dir().unwrap())
});
static SERVER_DATA_MANAGER: Lazy<RwLock<ServerDataManager>> =
    Lazy::new(|| RwLock::new(ServerDataManager::new(&FILESYSTEM_LAYOUT.session())));
static WEBSERVER_RUNTIME: Lazy<Mutex<Option<Runtime>>> =
    Lazy::new(|| Mutex::new(Runtime::new().ok()));

static STATISTICS_MANAGER: Lazy<Mutex<Option<StatisticsManager>>> = Lazy::new(|| Mutex::new(None));
static BITRATE_MANAGER: Lazy<Mutex<BitrateManager>> = Lazy::new(|| {
    let data_lock = SERVER_DATA_MANAGER.read();
    let settings = data_lock.settings();
    Mutex::new(BitrateManager::new(
        settings.connection.statistics_history_size as usize,
        settings.video.preferred_fps,
    ))
});

pub struct VideoPacket {
    pub header: VideoPacketHeader,
    pub payload: Vec<u8>,
}

static CONTROL_CHANNEL_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ServerControlPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static VIDEO_SENDER: Lazy<Mutex<Option<mpsc::Sender<VideoPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static HAPTICS_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Haptics>>>> =
    Lazy::new(|| Mutex::new(None));
static VIDEO_MIRROR_SENDER: Lazy<Mutex<Option<broadcast::Sender<Vec<u8>>>>> =
    Lazy::new(|| Mutex::new(None));
static VIDEO_RECORDING_FILE: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));

static DISCONNECT_CLIENT_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);
static RESTART_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

static FRAME_RENDER_VS_CSO: &[u8] = include_bytes!("../cpp/platform/win32/FrameRenderVS.cso");
static FRAME_RENDER_PS_CSO: &[u8] = include_bytes!("../cpp/platform/win32/FrameRenderPS.cso");
static QUAD_SHADER_CSO: &[u8] = include_bytes!("../cpp/platform/win32/QuadVertexShader.cso");
static COMPRESS_AXIS_ALIGNED_CSO: &[u8] =
    include_bytes!("../cpp/platform/win32/CompressAxisAlignedPixelShader.cso");
static COLOR_CORRECTION_CSO: &[u8] =
    include_bytes!("../cpp/platform/win32/ColorCorrectionPixelShader.cso");

static QUAD_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/quad.comp.spv");
static COLOR_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/color.comp.spv");
static FFR_SHADER_COMP_SPV: &[u8] = include_bytes!("../cpp/platform/linux/shader/ffr.comp.spv");
static RGBTOYUV420_SHADER_COMP_SPV: &[u8] =
    include_bytes!("../cpp/platform/linux/shader/rgbtoyuv420.comp.spv");

static DECODER_CONFIG: Lazy<Mutex<Option<DecoderInitializationConfig>>> =
    Lazy::new(|| Mutex::new(None));

fn to_ffi_quat(quat: Quat) -> FfiQuat {
    FfiQuat {
        x: quat.x,
        y: quat.y,
        z: quat.z,
        w: quat.w,
    }
}

pub fn create_recording_file() {
    let codec = SERVER_DATA_MANAGER.read().settings().video.preferred_codec;
    let ext = if matches!(codec, CodecType::H264) {
        "h264"
    } else {
        "h265"
    };

    let path = FILESYSTEM_LAYOUT.log_dir.join(format!("recording.{ext}"));

    match File::create(path) {
        Ok(mut file) => {
            if let Some(config) = &*DECODER_CONFIG.lock() {
                file.write_all(&config.config_buffer).ok();
            }

            *VIDEO_RECORDING_FILE.lock() = Some(file);

            unsafe { RequestIDR() };
        }
        Err(e) => {
            error!("Failed to record video on disk: {e}");
        }
    }
}

// This call is blocking
pub extern "C" fn shutdown_driver() {
    // Invoke connection runtimes shutdown
    // todo: block until they shutdown
    SHOULD_CONNECT_TO_CLIENTS.set(false);

    // apply openvr config for the next launch
    SERVER_DATA_MANAGER.write().session_mut().openvr_config = connection::contruct_openvr_config();

    if let Some(backup) = SERVER_DATA_MANAGER
        .write()
        .session_mut()
        .drivers_backup
        .take()
    {
        alvr_server_io::driver_registration(&backup.other_paths, true).ok();
        alvr_server_io::driver_registration(&[backup.alvr_path], false).ok();
    }

    WEBSERVER_RUNTIME.lock().take();

    unsafe { ShutdownSteamvr() };
}

pub fn notify_restart_driver() {
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    if system
        .processes_by_name(afs::dashboard_fname())
        .next()
        .is_some()
    {
        alvr_events::send_event(EventType::ServerRequestsSelfRestart);
    } else {
        error!("Cannot restart SteamVR. No dashboard process found on local device.");
    }
}

// This call is blocking
pub fn restart_driver() {
    SHOULD_CONNECT_TO_CLIENTS.set(false);
    RESTART_NOTIFIER.notify_waiters();

    // give time to the control loop to send the restart packet (not crucial)
    thread::sleep(Duration::from_millis(200));

    shutdown_driver();
}

fn init() {
    let (events_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    logging_backend::init_logging(events_sender.clone());

    if let Some(runtime) = WEBSERVER_RUNTIME.lock().as_mut() {
        runtime.spawn(alvr_common::show_err_async(web_server::web_server(
            events_sender,
        )));
    }

    {
        let mut data_manager_lock = SERVER_DATA_MANAGER.write();

        let connections = data_manager_lock.session().client_connections.clone();
        for (hostname, connection) in connections {
            if !connection.trusted {
                data_manager_lock.update_client_list(hostname, ClientListAction::RemoveEntry);
            }
        }

        for conn in data_manager_lock
            .session_mut()
            .client_connections
            .values_mut()
        {
            conn.current_ip = None;
        }
    }

    unsafe {
        g_sessionPath = CString::new(FILESYSTEM_LAYOUT.session().to_string_lossy().to_string())
            .unwrap()
            .into_raw();
        g_driverRootDir = CString::new(
            FILESYSTEM_LAYOUT
                .openvr_driver_root_dir
                .to_string_lossy()
                .to_string(),
        )
        .unwrap()
        .into_raw();
    };
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(init);

    FRAME_RENDER_VS_CSO_PTR = FRAME_RENDER_VS_CSO.as_ptr();
    FRAME_RENDER_VS_CSO_LEN = FRAME_RENDER_VS_CSO.len() as _;
    FRAME_RENDER_PS_CSO_PTR = FRAME_RENDER_PS_CSO.as_ptr();
    FRAME_RENDER_PS_CSO_LEN = FRAME_RENDER_PS_CSO.len() as _;
    QUAD_SHADER_CSO_PTR = QUAD_SHADER_CSO.as_ptr();
    QUAD_SHADER_CSO_LEN = QUAD_SHADER_CSO.len() as _;
    COMPRESS_AXIS_ALIGNED_CSO_PTR = COMPRESS_AXIS_ALIGNED_CSO.as_ptr();
    COMPRESS_AXIS_ALIGNED_CSO_LEN = COMPRESS_AXIS_ALIGNED_CSO.len() as _;
    COLOR_CORRECTION_CSO_PTR = COLOR_CORRECTION_CSO.as_ptr();
    COLOR_CORRECTION_CSO_LEN = COLOR_CORRECTION_CSO.len() as _;
    QUAD_SHADER_COMP_SPV_PTR = QUAD_SHADER_COMP_SPV.as_ptr();
    QUAD_SHADER_COMP_SPV_LEN = QUAD_SHADER_COMP_SPV.len() as _;
    COLOR_SHADER_COMP_SPV_PTR = COLOR_SHADER_COMP_SPV.as_ptr();
    COLOR_SHADER_COMP_SPV_LEN = COLOR_SHADER_COMP_SPV.len() as _;
    FFR_SHADER_COMP_SPV_PTR = FFR_SHADER_COMP_SPV.as_ptr();
    FFR_SHADER_COMP_SPV_LEN = FFR_SHADER_COMP_SPV.len() as _;
    RGBTOYUV420_SHADER_COMP_SPV_PTR = RGBTOYUV420_SHADER_COMP_SPV.as_ptr();
    RGBTOYUV420_SHADER_COMP_SPV_LEN = RGBTOYUV420_SHADER_COMP_SPV.len() as _;

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        alvr_common::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe extern "C" fn log_warn(string_ptr: *const c_char) {
        log(log::Level::Warn, string_ptr);
    }

    unsafe extern "C" fn log_info(string_ptr: *const c_char) {
        log(log::Level::Info, string_ptr);
    }

    unsafe extern "C" fn log_debug(string_ptr: *const c_char) {
        log(log::Level::Debug, string_ptr);
    }

    // Should not be used in production
    unsafe extern "C" fn log_periodically(tag_ptr: *const c_char, message_ptr: *const c_char) {
        const INTERVAL: Duration = Duration::from_secs(1);
        static LASTEST_TAG_TIMESTAMPS: Lazy<Mutex<HashMap<String, Instant>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));

        let tag = CStr::from_ptr(tag_ptr).to_string_lossy();
        let message = CStr::from_ptr(message_ptr).to_string_lossy();

        let mut timestamps_ref = LASTEST_TAG_TIMESTAMPS.lock();
        let old_timestamp = timestamps_ref
            .entry(tag.to_string())
            .or_insert_with(Instant::now);
        if *old_timestamp + INTERVAL < Instant::now() {
            *old_timestamp += INTERVAL;

            log::warn!("{}: {}", tag, message);
        }
    }

    extern "C" fn initialize_decoder(buffer_ptr: *const u8, len: i32, codec: i32) {
        let codec = if codec == 0 {
            CodecType::H264
        } else {
            CodecType::Hevc
        };

        let mut config_buffer = vec![0; len as usize];

        unsafe { ptr::copy_nonoverlapping(buffer_ptr, config_buffer.as_mut_ptr(), len as usize) };

        if let Some(sender) = &*VIDEO_MIRROR_SENDER.lock() {
            sender.send(config_buffer.clone()).ok();
        }

        if let Some(file) = &mut *VIDEO_RECORDING_FILE.lock() {
            file.write_all(&config_buffer).ok();
        }

        *DECODER_CONFIG.lock() = Some(DecoderInitializationConfig {
            codec,
            config_buffer,
        });
    }

    extern "C" fn video_send(timestamp_ns: u64, buffer_ptr: *mut u8, len: i32, is_idr: bool) {
        // start in the corrupts state, the client didn't receive the initial IDR yet.
        static STREAM_CORRUPTED: AtomicBool = AtomicBool::new(true);
        if let Some(sender) = &*VIDEO_SENDER.lock() {
            if is_idr {
                STREAM_CORRUPTED.store(false, Ordering::SeqCst);
            }

            let timestamp = Duration::from_nanos(timestamp_ns);

            let mut payload = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, payload.as_mut_ptr(), len as _);
            }

            if !STREAM_CORRUPTED.load(Ordering::SeqCst)
                || !SERVER_DATA_MANAGER
                    .read()
                    .settings()
                    .connection
                    .avoid_video_glitching
            {
                if let Some(sender) = &*VIDEO_MIRROR_SENDER.lock() {
                    sender.send(payload.clone()).ok();
                }

                if let Some(file) = &mut *VIDEO_RECORDING_FILE.lock() {
                    file.write_all(&payload).ok();
                }

                if matches!(
                    sender.try_send(VideoPacket {
                        header: VideoPacketHeader { timestamp, is_idr },
                        payload,
                    }),
                    Err(TrySendError::Full(_))
                ) {
                    STREAM_CORRUPTED.store(true, Ordering::SeqCst);
                    unsafe { crate::RequestIDR() };
                    warn!("Dropping video packet. Reason: Can't push to network");
                }
            } else {
                warn!("Dropping video packet. Reason: Waiting for IDR frame");
            }

            if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                stats.report_video_packet(len as _);
            }
            BITRATE_MANAGER
                .lock()
                .report_encoded_frame_size(timestamp, len as usize)
        }
    }

    extern "C" fn haptics_send(device_id: u64, duration_s: f32, frequency: f32, amplitude: f32) {
        if let Some(sender) = &*HAPTICS_SENDER.lock() {
            let haptics = Haptics {
                device_id,
                duration: Duration::from_secs_f32(f32::max(duration_s, 0.0)),
                frequency,
                amplitude,
            };

            sender.send(haptics).ok();
        }
    }

    pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
        SHOULD_CONNECT_TO_CLIENTS.set(true);

        thread::spawn(move || {
            if set_default_chap {
                // call this when inside a new tokio thread. Calling this on the parent thread will
                // crash SteamVR
                unsafe { SetChaperone(2.0, 2.0) };
            }

            if let Err(InterruptibleError::Other(e)) = connection::handshake_loop() {
                warn!("Connection thread closed: {e}");
            }
        });
    }

    unsafe extern "C" fn path_string_to_hash(path: *const c_char) -> u64 {
        alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
    }

    extern "C" fn report_present(timestamp_ns: u64, offset_ns: u64) {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_present(
                Duration::from_nanos(timestamp_ns),
                Duration::from_nanos(offset_ns),
            );
        }

        BITRATE_MANAGER.lock().report_frame_present(
            &SERVER_DATA_MANAGER
                .read()
                .settings()
                .video
                .bitrate
                .adapt_to_framerate,
        );
    }

    extern "C" fn report_composed(timestamp_ns: u64, offset_ns: u64) {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_composed(
                Duration::from_nanos(timestamp_ns),
                Duration::from_nanos(offset_ns),
            );
        }
    }

    extern "C" fn report_encoded(timestamp_ns: u64) {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_encoded(Duration::from_nanos(timestamp_ns));
        }
    }

    extern "C" fn get_dynamic_encoder_params() -> FfiDynamicEncoderParams {
        BITRATE_MANAGER
            .lock()
            .get_encoder_params(&SERVER_DATA_MANAGER.read().settings().video.bitrate)
    }

    extern "C" fn wait_for_vsync() {
        let wait_duration = STATISTICS_MANAGER
            .lock()
            .as_mut()
            .map(|stats| stats.duration_until_next_vsync());

        if let Some(duration) = wait_duration {
            thread::sleep(duration);
        }
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    LogPeriodically = Some(log_periodically);
    DriverReadyIdle = Some(driver_ready_idle);
    InitializeDecoder = Some(initialize_decoder);
    VideoSend = Some(video_send);
    HapticsSend = Some(haptics_send);
    ShutdownRuntime = Some(shutdown_driver);
    PathStringToHash = Some(path_string_to_hash);
    ReportPresent = Some(report_present);
    ReportComposed = Some(report_composed);
    ReportEncoded = Some(report_encoded);
    GetSerialNumber = Some(openvr_props::get_serial_number);
    SetOpenvrProps = Some(openvr_props::set_device_openvr_props);
    GetDynamicEncoderParams = Some(get_dynamic_encoder_params);
    WaitForVSync = Some(wait_for_vsync);

    // cast to usize to allow the variables to cross thread boundaries
    let interface_name_usize = interface_name as usize;
    let return_code_usize = return_code as usize;

    static PTR_USIZE: OnceCell<AtomicUsize> = OnceCell::new();
    static NUM_TRIALS: OnceCell<AtomicUsize> = OnceCell::new();

    PTR_USIZE.set(AtomicUsize::new(0)).ok();
    NUM_TRIALS.set(AtomicUsize::new(0)).ok();

    thread::spawn(move || {
        NUM_TRIALS.get().unwrap().fetch_add(1, Ordering::Relaxed);
        if NUM_TRIALS.get().unwrap().load(Ordering::Relaxed) <= 1 {
            PTR_USIZE.get().unwrap().store(
                CppEntryPoint(interface_name_usize as _, return_code_usize as _) as _,
                Ordering::Relaxed,
            );
        }
    })
    .join()
    .ok();

    PTR_USIZE.get().unwrap().load(Ordering::Relaxed) as _
}
