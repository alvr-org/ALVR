mod connection;
mod connection_utils;
mod dashboard;
mod graphics_info;
mod logging_backend;
mod web_server;

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    non_snake_case
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use alvr_common::{lazy_static, log, parking_lot::Mutex, prelude::*, ALVR_VERSION};
use alvr_filesystem::{self as afs, Layout};
use alvr_session::{
    ClientConnectionDesc, OpenvrPropValue, OpenvrPropertyKey, ServerEvent, SessionManager,
};
use alvr_sockets::{Haptics, TimeSyncPacket, VideoFrameHeaderPacket};
use graphics_info::GpuVendor;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Once,
    },
    thread,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, mpsc, Notify},
};

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref FILESYSTEM_LAYOUT: Layout =
        afs::filesystem_layout_from_openvr_driver_root_dir(&alvr_commands::get_driver_dir().unwrap());
    static ref SESSION_MANAGER: Mutex<SessionManager> =
        Mutex::new(SessionManager::new(&FILESYSTEM_LAYOUT.session()));
    static ref RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref MAYBE_WINDOW: Mutex<Option<Arc<alcro::UI>>> = Mutex::new(None);

    static ref VIDEO_SENDER: Mutex<Option<mpsc::UnboundedSender<(VideoFrameHeaderPacket, Vec<u8>)>>> =
        Mutex::new(None);
    static ref HAPTICS_SENDER: Mutex<Option<mpsc::UnboundedSender<Haptics>>> =
        Mutex::new(None);
    static ref TIME_SYNC_SENDER: Mutex<Option<mpsc::UnboundedSender<TimeSyncPacket>>> =
        Mutex::new(None);

    static ref CLIENTS_UPDATED_NOTIFIER: Notify = Notify::new();
    static ref RESTART_NOTIFIER: Notify = Notify::new();
    static ref SHUTDOWN_NOTIFIER: Notify = Notify::new();

    static ref FRAME_RENDER_VS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderVS.cso").to_vec();
    static ref FRAME_RENDER_PS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderPS.cso").to_vec();
    static ref QUAD_SHADER_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/QuadVertexShader.cso").to_vec();
    static ref COMPRESS_AXIS_ALIGNED_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/CompressAxisAlignedPixelShader.cso").to_vec();
    static ref COLOR_CORRECTION_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/ColorCorrectionPixelShader.cso").to_vec();
}

pub fn to_cpp_openvr_prop(key: OpenvrPropertyKey, value: OpenvrPropValue) -> OpenvrProperty {
    let type_ = match value {
        OpenvrPropValue::Bool(_) => OpenvrPropertyType_Bool,
        OpenvrPropValue::Float(_) => OpenvrPropertyType_Float,
        OpenvrPropValue::Int32(_) => OpenvrPropertyType_Int32,
        OpenvrPropValue::Uint64(_) => OpenvrPropertyType_Uint64,
        OpenvrPropValue::Vector3(_) => OpenvrPropertyType_Vector3,
        OpenvrPropValue::Double(_) => OpenvrPropertyType_Double,
        OpenvrPropValue::String(_) => OpenvrPropertyType_String,
    };

    let value = match value {
        OpenvrPropValue::Bool(bool_) => OpenvrPropertyValue { bool_ },
        OpenvrPropValue::Float(float_) => OpenvrPropertyValue { float_ },
        OpenvrPropValue::Int32(int32) => OpenvrPropertyValue { int32 },
        OpenvrPropValue::Uint64(uint64) => OpenvrPropertyValue { uint64 },
        OpenvrPropValue::Vector3(vector3) => OpenvrPropertyValue { vector3 },
        OpenvrPropValue::Double(double_) => OpenvrPropertyValue { double_ },
        OpenvrPropValue::String(value) => {
            let c_string = CString::new(value).unwrap();
            let mut string = [0; 64];

            unsafe {
                ptr::copy_nonoverlapping(
                    c_string.as_ptr(),
                    string.as_mut_ptr(),
                    c_string.as_bytes_with_nul().len(),
                );
            }

            OpenvrPropertyValue { string }
        }
    };

    OpenvrProperty {
        key: key as u32,
        type_,
        value,
    }
}

pub fn shutdown_runtime() {
    alvr_session::log_event(ServerEvent::ServerQuitting);

    if let Some(window) = MAYBE_WINDOW.lock().take() {
        window.close();
    }

    SHUTDOWN_NOTIFIER.notify_waiters();

    if let Some(runtime) = RUNTIME.lock().take() {
        runtime.shutdown_background();
        // shutdown_background() is non blocking and it does not guarantee that every internal
        // thread is terminated in a timely manner. Using shutdown_background() instead of just
        // dropping the runtime has the benefit of giving SteamVR a chance to clean itself as
        // much as possible before the process is killed because of alvr_launcher timeout.
    }
}

pub fn notify_shutdown_driver() {
    thread::spawn(|| {
        RESTART_NOTIFIER.notify_waiters();

        // give time to the control loop to send the restart packet (not crucial)
        thread::sleep(Duration::from_millis(100));

        shutdown_runtime();

        unsafe { ShutdownSteamvr() };
    });
}

pub fn notify_restart_driver() {
    notify_shutdown_driver();

    alvr_commands::restart_steamvr(&FILESYSTEM_LAYOUT.launcher_exe()).ok();
}

pub fn notify_application_update() {
    notify_shutdown_driver();

    alvr_commands::invoke_application_update(&FILESYSTEM_LAYOUT.launcher_exe()).ok();
}

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub fn update_client_list(hostname: String, action: ClientListAction) {
    let mut client_connections = SESSION_MANAGER.lock().get().client_connections.clone();

    let maybe_client_entry = client_connections.entry(hostname);

    let mut updated = false;
    match action {
        ClientListAction::AddIfMissing { display_name } => {
            if let Entry::Vacant(new_entry) = maybe_client_entry {
                let client_connection_desc = ClientConnectionDesc {
                    trusted: false,
                    manual_ips: HashSet::new(),
                    display_name,
                };
                new_entry.insert(client_connection_desc);

                updated = true;
            }
        }
        ClientListAction::TrustAndMaybeAddIp(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                let client_connection_ref = entry.get_mut();
                client_connection_ref.trusted = true;
                if let Some(ip) = maybe_ip {
                    client_connection_ref.manual_ips.insert(ip);
                }

                updated = true;
            }
            // else: never happens. The function must be called with AddIfMissing{} first
        }
        ClientListAction::RemoveIpOrEntry(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                if let Some(ip) = maybe_ip {
                    entry.get_mut().manual_ips.remove(&ip);
                } else {
                    entry.remove_entry();
                }

                updated = true;
            }
        }
    }

    if updated {
        SESSION_MANAGER.lock().get_mut().client_connections = client_connections;

        CLIENTS_UPDATED_NOTIFIER.notify_waiters();
    }
}

fn init() {
    let (log_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    let (events_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    logging_backend::init_logging(log_sender.clone(), events_sender.clone());

    if let Some(runtime) = RUNTIME.lock().as_mut() {
        // Acquire and drop the session_manager lock to create session.json if not present
        // this is needed until Settings.cpp is replaced with Rust. todo: remove
        SESSION_MANAGER.lock().get_mut();

        runtime.spawn(async move {
            let connections = SESSION_MANAGER.lock().get().client_connections.clone();
            for (hostname, connection) in connections {
                if !connection.trusted {
                    update_client_list(hostname, ClientListAction::RemoveIpOrEntry(None));
                }
            }

            let web_server = alvr_common::show_err_async(web_server::web_server(
                log_sender,
                events_sender.clone(),
            ));

            tokio::select! {
                _ = web_server => (),
                _ = SHUTDOWN_NOTIFIER.notified() => (),
            }
        });

        thread::spawn(|| alvr_common::show_err(dashboard::ui_thread()));
    }

    if matches!(graphics_info::get_gpu_vendor(), GpuVendor::Nvidia) {
        SESSION_MANAGER
            .lock()
            .get_mut()
            .session_settings
            .video
            .linux_async_reprojection = false;
    }

    if SESSION_MANAGER.lock().get().server_version != *ALVR_VERSION {
        let mut session_manager = SESSION_MANAGER.lock();
        let mut session_ref = session_manager.get_mut();
        session_ref.server_version = ALVR_VERSION.clone();
        session_ref.client_connections.clear();
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

    extern "C" fn video_send(header: VideoFrame, buffer_ptr: *mut u8, len: i32) {
        if let Some(sender) = &*VIDEO_SENDER.lock() {
            let header = VideoFrameHeaderPacket {
                packet_counter: header.packetCounter,
                tracking_frame_index: header.trackingFrameIndex,
                video_frame_index: header.videoFrameIndex,
                sent_time: header.sentTime,
                frame_byte_size: header.frameByteSize,
                fec_index: header.fecIndex,
                fec_percentage: header.fecPercentage,
            };

            let mut vec_buffer = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, vec_buffer.as_mut_ptr(), len as _);
            }

            sender.send((header, vec_buffer)).ok();
        }
    }

    extern "C" fn haptics_send(path: u64, duration_s: f32, frequency: f32, amplitude: f32) {
        if let Some(sender) = &*HAPTICS_SENDER.lock() {
            let haptics = Haptics {
                path,
                duration: Duration::from_secs_f32(duration_s),
                frequency,
                amplitude,
            };

            sender.send(haptics).ok();
        }
    }

    extern "C" fn time_sync_send(data: TimeSync) {
        if let Some(sender) = &*TIME_SYNC_SENDER.lock() {
            let time_sync = TimeSyncPacket {
                mode: data.mode,
                server_time: data.serverTime,
                client_time: data.clientTime,
                packets_lost_total: data.packetsLostTotal,
                packets_lost_in_second: data.packetsLostInSecond,
                average_send_latency: data.averageSendLatency,
                average_transport_latency: data.averageTransportLatency,
                average_decode_latency: data.averageDecodeLatency,
                idle_time: data.idleTime,
                fec_failure: data.fecFailure,
                fec_failure_in_second: data.fecFailureInSecond,
                fec_failure_total: data.fecFailureTotal,
                fps: data.fps,
                server_total_latency: data.serverTotalLatency,
                tracking_recv_frame_index: data.trackingRecvFrameIndex,
            };

            sender.send(time_sync).ok();
        }
    }

    pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
        alvr_common::show_err(alvr_commands::apply_driver_paths_backup(
            FILESYSTEM_LAYOUT.openvr_driver_root_dir.clone(),
        ));

        if let Some(runtime) = &mut *RUNTIME.lock() {
            runtime.spawn(async move {
                if set_default_chap {
                    // call this when inside a new tokio thread. Calling this on the parent thread will
                    // crash SteamVR
                    unsafe { SetChaperone(2.0, 2.0) };
                }
                tokio::select! {
                    _ = connection::connection_lifecycle_loop() => (),
                    _ = SHUTDOWN_NOTIFIER.notified() => (),
                }
            });
        }
    }

    extern "C" fn _shutdown_runtime() {
        shutdown_runtime();
    }

    unsafe extern "C" fn path_string_to_hash(path: *const c_char) -> u64 {
        alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    DriverReadyIdle = Some(driver_ready_idle);
    VideoSend = Some(video_send);
    HapticsSend = Some(haptics_send);
    TimeSyncSend = Some(time_sync_send);
    ShutdownRuntime = Some(_shutdown_runtime);
    PathStringToHash = Some(path_string_to_hash);

    // cast to usize to allow the variables to cross thread boundaries
    let interface_name_usize = interface_name as usize;
    let return_code_usize = return_code as usize;

    lazy_static! {
        static ref MAYBE_PTR_USIZE: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        static ref NUM_TRIALS: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    }

    thread::spawn(move || {
        NUM_TRIALS.fetch_add(1, Ordering::Relaxed);
        if NUM_TRIALS.load(Ordering::Relaxed) <= 1 {
            MAYBE_PTR_USIZE.store(
                CppEntryPoint(interface_name_usize as _, return_code_usize as _) as _,
                Ordering::Relaxed,
            );
        }
    })
    .join()
    .ok();

    MAYBE_PTR_USIZE.load(Ordering::Relaxed) as _
}
