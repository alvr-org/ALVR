#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

mod connection;
mod logging_backend;
mod statistics_manager;
mod web_server;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{commands::*, data::*, logging::*, sockets::*, *};
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use statistics_manager::StatisticsManager;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    path::PathBuf,
    slice,
    sync::{Arc, Once},
    thread,
    time::{Duration, SystemTime},
};
use tokio::{runtime::Runtime, sync::broadcast};

pub type AMutex<T> = tokio::sync::Mutex<T>;

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref ALVR_DIR: PathBuf = get_alvr_dir().unwrap();

    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref MAYBE_SHUTDOWN_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
    static ref MAYBE_VIDEO_SENDER: Mutex<Option<StreamSender<VideoPacket>>> = Mutex::new(None);
    static ref MAYBE_AUDIO_SENDER: Mutex<Option<StreamSender<AudioPacket>>> = Mutex::new(None);
    static ref MAYBE_HAPTICS_SENDER: Mutex<Option<StreamSender<HapticsPacket>>> = Mutex::new(None);
    static ref STATISTICS: Mutex<StatisticsManager> = Mutex::new(StatisticsManager::new());
}

pub fn shutdown_runtime() {
    if let Some(notifier) = &*MAYBE_SHUTDOWN_NOTIFIER.lock() {
        notifier.send(()).ok();
    }

    if let Some(runtime) = MAYBE_RUNTIME.lock().take() {
        runtime.shutdown_background();
        // shutdown_background() is non blocking and it does not guarantee that every internal
        // thread is terminated in a timely manner. Using shutdown_background() instead of just
        // dropping the runtime has the benefit of giving SteamVR a chance to clean itself as
        // much as possible before the process is killed because of alvr_launcher timeout.
    }
}

pub fn restart_steamvr() {
    thread::spawn(|| {
        shutdown_runtime();

        unsafe { ShutdownSteamvr() };

        restart_steamvr_with_timeout(&ALVR_DIR).ok();
    });
}

pub enum ClientListAction {
    AddIfMissing { ip: IpAddr, certificate_pem: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub async fn update_client_list(
    session_manager: Arc<AMutex<SessionManager>>,
    hostname: String,
    action: ClientListAction,
    update_client_listeners_notifier: broadcast::Sender<()>,
) {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let session_manager_ref = &mut session_manager.lock().await;
    let session_desc_ref =
        &mut session_manager_ref.get_mut(None, SessionUpdateType::ClientList);

    let maybe_client_entry = session_desc_ref.client_connections.entry(hostname);

    match action {
        ClientListAction::AddIfMissing {
            ip,
            certificate_pem,
        } => match maybe_client_entry {
            Entry::Occupied(mut existing_entry) => {
                let client_connection_ref = existing_entry.get_mut();
                client_connection_ref.last_update_ms_since_epoch = now_ms as _;
                client_connection_ref.last_ip = ip;
            }
            Entry::Vacant(new_entry) => {
                let client_connection_desc = ClientConnectionDesc {
                    trusted: false,
                    last_update_ms_since_epoch: now_ms as _,
                    last_ip: ip,
                    manual_ips: HashSet::new(),
                    display_name: None,
                    certificate_pem,
                };
                new_entry.insert(client_connection_desc);

                info!(id: LogId::SessionUpdated {
                    web_client_id: None,
                    update_type: SessionUpdateType::ClientList
                });
            }
        },
        ClientListAction::TrustAndMaybeAddIp(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                let client_connection_ref = entry.get_mut();
                client_connection_ref.trusted = true;
                if let Some(ip) = maybe_ip {
                    client_connection_ref.manual_ips.insert(ip);
                }

                info!(id: LogId::SessionUpdated {
                    web_client_id: None,
                    update_type: SessionUpdateType::ClientList
                });
            }
            // else: never happens. The UI cannot request a new entry creation because in that case
            // it wouldn't have the certificate
        }
        ClientListAction::RemoveIpOrEntry(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                if let Some(ip) = maybe_ip {
                    entry.get_mut().manual_ips.remove(&ip);
                } else {
                    entry.remove_entry();
                }

                info!(id: LogId::SessionUpdated {
                    web_client_id: None,
                    update_type: SessionUpdateType::ClientList
                });
            }
        }
    }

    if let Err(e) = update_client_listeners_notifier.send(()) {
        warn!("Failed to notify client list update: {:?}", e);
    }
}

fn init(log_sender: broadcast::Sender<String>) -> StrResult {
    if let Some(runtime) = MAYBE_RUNTIME.lock().as_mut() {
        let session_manager = Arc::new(AMutex::new(SessionManager::new(&ALVR_DIR)));

        // this is needed until all c++ code is rewritten. todo: remove
        runtime
            .block_on(session_manager.lock())
            .get_mut(None, SessionUpdateType::Other);

        let (shutdown_notifier, mut shutdown_receiver) = broadcast::channel(1);
        let (update_client_listeners_notifier, _) = broadcast::channel(1);

        runtime.spawn(async move {
            let web_server = show_err_async(web_server::web_server(
                session_manager.clone(),
                log_sender,
                update_client_listeners_notifier.clone(),
            ));

            // let connection_loop = show_err_async(connection::connection_loop(
            //     session_manager,
            //     update_client_listeners_notifier,
            // ));

            tokio::select! {
                _ = web_server => (),
                // _ = connection_loop => (),
                _ = shutdown_receiver.recv() => (),
            }
        });

        *MAYBE_SHUTDOWN_NOTIFIER.lock() = Some(shutdown_notifier);
    }

    let alvr_dir_c_string = CString::new(ALVR_DIR.to_string_lossy().into_owned()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };

    // ALVR_DIR has been used (and so initialized). I don't need alvr_dir storage on disk anymore
    maybe_delete_alvr_dir_storage();

    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(|| {
        let (log_sender, _) = broadcast::channel(web_server::LOG_BROADCAST_CAPACITY);
        logging_backend::init_logging(log_sender.clone());

        show_err(init(log_sender)).ok();
    });

    lazy_static_include_bytes!(FRAME_RENDER_VS_CSO => "cpp/alvr_server/FrameRenderVS.cso");
    lazy_static_include_bytes!(FRAME_RENDER_PS_CSO => "cpp/alvr_server/FrameRenderPS.cso");
    lazy_static_include_bytes!(QUAD_SHADER_CSO  => "cpp/alvr_server/QuadVertexShader.cso");
    lazy_static_include_bytes!(COMPRESS_SLICES_CSO => "cpp/alvr_server/CompressSlicesPixelShader.cso");
    lazy_static_include_bytes!(COLOR_CORRECTION_CSO => "cpp/alvr_server/ColorCorrectionPixelShader.cso");

    FRAME_RENDER_VS_CSO_PTR = FRAME_RENDER_VS_CSO.as_ptr();
    FRAME_RENDER_VS_CSO_LEN = FRAME_RENDER_VS_CSO.len() as _;
    FRAME_RENDER_PS_CSO_PTR = FRAME_RENDER_PS_CSO.as_ptr();
    FRAME_RENDER_PS_CSO_LEN = FRAME_RENDER_PS_CSO.len() as _;
    QUAD_SHADER_CSO_PTR = QUAD_SHADER_CSO.as_ptr();
    QUAD_SHADER_CSO_LEN = QUAD_SHADER_CSO.len() as _;
    COMPRESS_SLICES_CSO_PTR = COMPRESS_SLICES_CSO.as_ptr();
    COMPRESS_SLICES_CSO_LEN = COMPRESS_SLICES_CSO.len() as _;
    COLOR_CORRECTION_CSO_PTR = COLOR_CORRECTION_CSO.as_ptr();
    COLOR_CORRECTION_CSO_LEN = COLOR_CORRECTION_CSO.len() as _;

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        show_e(CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
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

    unsafe extern "C" fn send_video(
        packet_index: u64,
        raw_buffer: *mut u8,
        len: i32,
        tracking_index: u64,
    ) {
        if let (Some(runtime), Some(sender)) =
            (&mut *MAYBE_RUNTIME.lock(), &mut *MAYBE_VIDEO_SENDER.lock())
        {
            let buf_slice = slice::from_raw_parts(raw_buffer, len as _);

            // use block_on() instead of spawn() because "sender" must remain valid
            runtime.block_on(async move {
                let res = sender
                    .send(&VideoPacket {
                        packet_index,
                        tracking_index,
                        buffer: buf_slice.to_vec(),
                    })
                    .await;
                if let Err(e) = res {
                    debug!("Failed to send video packet: {}", e);
                }
            });
        }
    }

    unsafe extern "C" fn send_audio(
        packet_index: u64,
        raw_buffer: *mut u8,
        len: i32,
        presentation_time_us: u64,
    ) {
        if let (Some(runtime), Some(sender)) =
            (&mut *MAYBE_RUNTIME.lock(), &mut *MAYBE_AUDIO_SENDER.lock())
        {
            let buf_slice = slice::from_raw_parts(raw_buffer, len as _);

            runtime.block_on(async move {
                let res = sender
                    .send(&AudioPacket {
                        packet_index,
                        presentation_time: Duration::from_micros(presentation_time_us),
                        buffer: buf_slice.to_vec(),
                    })
                    .await;
                if let Err(e) = res {
                    debug!("Failed to send video packet: {}", e);
                }
            });
        }
    }

    unsafe extern "C" fn send_haptics(amplitude: f32, duration: f32, frequency: f32, hand: u8) {
        if let (Some(runtime), Some(sender)) = (
            &mut *MAYBE_RUNTIME.lock(),
            &mut *MAYBE_HAPTICS_SENDER.lock(),
        ) {
            runtime.block_on(async move {
                let res = sender
                    .send(&HapticsPacket {
                        amplitude,
                        duration,
                        frequency,
                        device: if hand == 0 {
                            TrackedDeviceType::LeftController
                        } else {
                            TrackedDeviceType::RightController
                        },
                    })
                    .await;
                if let Err(e) = res {
                    debug!("Failed to send video packet: {}", e);
                }
            });
        }
    }

    unsafe extern "C" fn report_encode_latency(latency_us: u64) {
        STATISTICS
            .lock()
            .report_encode_latency(Duration::from_micros(latency_us));
    }

    extern "C" fn _shutdown_runtime() {
        shutdown_runtime();
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    SendVideo = Some(send_video);
    SendAudio = Some(send_audio);
    SendHapticsFeedback = Some(send_haptics);
    ReportEncodeLatency = Some(report_encode_latency);
    ShutdownRuntime = Some(_shutdown_runtime);

    CppEntryPoint(interface_name, return_code)
}
