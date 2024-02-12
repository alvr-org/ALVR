mod bitrate;
mod body_tracking;
mod c_api;
mod connection;
mod face_tracking;
mod hand_gestures;
mod haptics;
mod input_mapping;
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
    error,
    glam::Quat,
    log,
    once_cell::sync::Lazy,
    parking_lot::{Mutex, RwLock},
    ConnectionState, LifecycleState, OptLazy, RelaxedAtomic,
};
use alvr_events::EventType;
use alvr_filesystem::{self as afs, Layout};
use alvr_packets::{ClientListAction, DecoderInitializationConfig, VideoPacketHeader};
use alvr_server_io::ServerDataManager;
use alvr_session::{CodecType, Settings};
use bitrate::BitrateManager;
use statistics::StatisticsManager;
use std::{
    collections::HashMap,
    env,
    ffi::{c_char, c_void, CStr, CString},
    fs::File,
    io::Write,
    ptr,
    sync::Once,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, RefreshKind};
use tokio::{runtime::Runtime, sync::broadcast};

pub static LIFECYCLE_STATE: RwLock<LifecycleState> = RwLock::new(LifecycleState::StartingUp);
pub static IS_RESTARTING: RelaxedAtomic = RelaxedAtomic::new(false);
static CONNECTION_THREAD: RwLock<Option<JoinHandle<()>>> = RwLock::new(None);

static FILESYSTEM_LAYOUT: Lazy<Layout> = Lazy::new(|| {
    afs::filesystem_layout_from_openvr_driver_root_dir(
        &alvr_server_io::get_driver_dir_from_registered().unwrap(),
    )
});
static SERVER_DATA_MANAGER: Lazy<RwLock<ServerDataManager>> =
    Lazy::new(|| RwLock::new(ServerDataManager::new(&FILESYSTEM_LAYOUT.session())));
static WEBSERVER_RUNTIME: OptLazy<Runtime> = Lazy::new(|| Mutex::new(Runtime::new().ok()));

static STATISTICS_MANAGER: OptLazy<StatisticsManager> = alvr_common::lazy_mut_none();
static BITRATE_MANAGER: Lazy<Mutex<BitrateManager>> =
    Lazy::new(|| Mutex::new(BitrateManager::new(256, 60.0)));

pub struct VideoPacket {
    pub header: VideoPacketHeader,
    pub payload: Vec<u8>,
}

static VIDEO_MIRROR_SENDER: OptLazy<broadcast::Sender<Vec<u8>>> = alvr_common::lazy_mut_none();
static VIDEO_RECORDING_FILE: OptLazy<File> = alvr_common::lazy_mut_none();

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

static DECODER_CONFIG: OptLazy<DecoderInitializationConfig> = alvr_common::lazy_mut_none();

fn to_ffi_quat(quat: Quat) -> FfiQuat {
    FfiQuat {
        x: quat.x,
        y: quat.y,
        z: quat.z,
        w: quat.w,
    }
}

pub fn create_recording_file(settings: &Settings) {
    let codec = settings.video.preferred_codec;
    let ext = match codec {
        CodecType::H264 => "h264",
        CodecType::Hevc => "h265",
        CodecType::AV1 => "av1",
    };

    let path = FILESYSTEM_LAYOUT.log_dir.join(format!(
        "recording.{}.{ext}",
        chrono::Local::now().format("%F.%H-%M-%S")
    ));

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
    *LIFECYCLE_STATE.write() = LifecycleState::ShuttingDown;

    {
        let mut data_manager_lock = SERVER_DATA_MANAGER.write();

        let hostnames = data_manager_lock
            .client_list()
            .iter()
            .filter(|&(_, info)| {
                !matches!(
                    info.connection_state,
                    ConnectionState::Disconnected | ConnectionState::Disconnecting { .. }
                )
            })
            .map(|(hostname, _)| hostname.clone())
            .collect::<Vec<_>>();

        for hostname in hostnames {
            data_manager_lock.update_client_list(
                hostname,
                ClientListAction::SetConnectionState(ConnectionState::Disconnecting),
            );
        }
    }

    if let Some(thread) = CONNECTION_THREAD.write().take() {
        thread.join().ok();
    }

    // apply openvr config for the next launch
    {
        let mut server_data_lock = SERVER_DATA_MANAGER.write();
        server_data_lock.session_mut().openvr_config =
            connection::contruct_openvr_config(server_data_lock.session());
    }

    if let Some(backup) = SERVER_DATA_MANAGER
        .write()
        .session_mut()
        .drivers_backup
        .take()
    {
        alvr_server_io::driver_registration(&backup.other_paths, true).ok();
        alvr_server_io::driver_registration(&[backup.alvr_path], false).ok();
    }

    while SERVER_DATA_MANAGER
        .read()
        .client_list()
        .iter()
        .any(|(_, info)| info.connection_state != ConnectionState::Disconnected)
    {
        thread::sleep(Duration::from_millis(100));
    }

    #[cfg(target_os = "windows")]
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
    IS_RESTARTING.set(true);

    shutdown_driver();
}

fn init() {
    let (events_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    logging_backend::init_logging(events_sender.clone());

    if SERVER_DATA_MANAGER
        .read()
        .settings()
        .logging
        .prefer_backtrace
    {
        env::set_var("RUST_BACKTRACE", "1");
    }

    SERVER_DATA_MANAGER.write().clean_client_list();

    if let Some(runtime) = WEBSERVER_RUNTIME.lock().as_mut() {
        runtime.spawn(async { alvr_common::show_err(web_server::web_server(events_sender).await) });
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

    extern "C" fn set_video_config_nals(buffer_ptr: *const u8, len: i32, codec: i32) {
        let codec = if codec == 0 {
            CodecType::H264
        } else if codec == 1 {
            CodecType::Hevc
        } else {
            CodecType::AV1
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

    pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
        // Note: Idle state is not used on the server side
        *LIFECYCLE_STATE.write() = LifecycleState::Resumed;

        thread::spawn(move || {
            if set_default_chap {
                // call this when inside a new thread. Calling this on the parent thread will crash
                // SteamVR
                unsafe {
                    InitOpenvrClient();
                    SetChaperoneArea(2.0, 2.0);
                    ShutdownOpenvrClient();
                }
            }

            connection::handshake_loop();
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

        let server_data_lock = SERVER_DATA_MANAGER.read();
        BITRATE_MANAGER
            .lock()
            .report_frame_present(&server_data_lock.settings().video.bitrate.adapt_to_framerate);
    }

    extern "C" fn report_composed(timestamp_ns: u64, offset_ns: u64) {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_composed(
                Duration::from_nanos(timestamp_ns),
                Duration::from_nanos(offset_ns),
            );
        }
    }

    extern "C" fn get_dynamic_encoder_params() -> FfiDynamicEncoderParams {
        let (params, stats) = {
            let server_data_lock = SERVER_DATA_MANAGER.read();
            BITRATE_MANAGER
                .lock()
                .get_encoder_params(&server_data_lock.settings().video.bitrate)
        };

        if let Some(stats) = stats {
            if let Some(stats_manager) = &mut *STATISTICS_MANAGER.lock() {
                stats_manager.report_nominal_bitrate_stats(stats);
            }
        }

        params
    }

    extern "C" fn wait_for_vsync() {
        if SERVER_DATA_MANAGER
            .read()
            .settings()
            .video
            .optimize_game_render_latency
        {
            // Note: unlock STATISTICS_MANAGER as soon as possible
            let wait_duration = STATISTICS_MANAGER
                .lock()
                .as_mut()
                .map(|stats| stats.duration_until_next_vsync());

            if let Some(duration) = wait_duration {
                thread::sleep(duration);
            }
        }
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    LogPeriodically = Some(log_periodically);
    DriverReadyIdle = Some(driver_ready_idle);
    SetVideoConfigNals = Some(set_video_config_nals);
    VideoSend = Some(connection::send_video);
    HapticsSend = Some(connection::send_haptics);
    ShutdownRuntime = Some(shutdown_driver);
    PathStringToHash = Some(path_string_to_hash);
    ReportPresent = Some(report_present);
    ReportComposed = Some(report_composed);
    GetSerialNumber = Some(openvr_props::get_serial_number);
    SetOpenvrProps = Some(openvr_props::set_device_openvr_props);
    RegisterButtons = Some(input_mapping::register_buttons);
    GetDynamicEncoderParams = Some(get_dynamic_encoder_params);
    WaitForVSync = Some(wait_for_vsync);

    CppEntryPoint(interface_name, return_code)
}
