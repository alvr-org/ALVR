#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals)]

mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, process::*, sockets::*, *};
use futures::{future, SinkExt};
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use std::{
    convert::Infallible,
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    path::PathBuf,
    sync::{atomic::*, Arc, Once},
    thread,
    time::SystemTime,
};
use tokio::{
    runtime::Runtime,
    stream::StreamExt,
    sync::broadcast::{self, RecvError},
};
use warp::{
    body, fs as wfs,
    http::StatusCode,
    reply,
    ws::{Message, WebSocket, Ws},
    Filter, Reply,
};

type AMutex<T> = tokio::sync::Mutex<T>;

const WEB_GUI_DIR_STR: &str = "web_gui";
const LOG_BROADCAST_CAPACITY: usize = 256;

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref MAYBE_SHUTDOWN_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
}

fn shutdown_runtime() {
    if let Some(notifier) = &*MAYBE_SHUTDOWN_NOTIFIER.lock() {
        notifier.send(()).ok();
    }

    if let Some(runtime) = MAYBE_RUNTIME.lock().take() {
        runtime.shutdown_background();
        // shutdown_background() is non blocking and it does not guarantee that every internal
        // thread is terminated in a timely manner. Using shutdown_background() instead of just
        // dropping the runtime has the benefit of giving SteamVR a chance to clean itself as
        // much as possible before the process is killed because of alvr_bootstrap timeout.
    }
}

fn restart_steamvr() {
    thread::spawn(|| {
        shutdown_runtime();

        unsafe {
            if let Some(shutdown_steamvr_callback) = ShutdownSteamvr {
                shutdown_steamvr_callback();
            }
        }

        // todo: launch alvr_bootstrap
    });
}

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

async fn subscribed_to_log(mut socket: WebSocket, mut log_receiver: broadcast::Receiver<String>) {
    while let Some(maybe_line) = log_receiver.next().await {
        match maybe_line {
            Ok(line) => {
                if let Err(e) = socket.send(Message::text(line)).await {
                    info!("Failed to send log with websocket: {}", e);
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => {
                warn!("Some log lines have been lost because the buffer is full");
            }
            Err(RecvError::Closed) => break,
        }
    }
}

async fn set_session_handler(
    session_manager: Arc<AMutex<SessionManager>>,
    update_type: String,
    update_author_id: String,
    value: serde_json::Value,
) -> Result<impl Reply, Infallible> {
    if let Ok(update_type) = serde_json::from_str(&format!("\"{}\"", update_type)) {
        let res = session_manager
            .lock()
            .await
            .get_mut(&update_author_id, update_type)
            .merge_from_json(value);

        if let Err(e) = res {
            warn!("{}", e);
            // HTTP Code: WARNING
            Ok(reply::with_status(
                reply(),
                StatusCode::from_u16(199).unwrap(),
            ))
        } else {
            Ok(reply::with_status(reply(), StatusCode::OK))
        }
    } else {
        Ok(reply::with_status(reply(), StatusCode::BAD_REQUEST))
    }
}

async fn web_server(
    session_manager: Arc<AMutex<SessionManager>>,
    log_sender: broadcast::Sender<String>,
    mut shutdown_receiver: broadcast::Receiver<()>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    let settings_changed = Arc::new(AtomicBool::new(false));

    let web_gui_dir = PathBuf::from(WEB_GUI_DIR_STR);
    let index_request = warp::path::end().and(wfs::file(web_gui_dir.join("index.html")));
    let files_requests = wfs::dir(web_gui_dir);

    let settings_schema_request = warp::path("settings-schema")
        .map(|| reply::json(&settings_schema(settings_cache_default())));

    let session_requests =
        warp::path("session").and(
            warp::get()
                .and_then({
                    let session_manager = session_manager.clone();
                    move || {
                        let session_manager = session_manager.clone();
                        async move {
                            Ok::<_, Infallible>(reply::json(session_manager.lock().await.get()))
                        }
                    }
                })
                .or(warp::path!(String / String).and(body::json()).and_then({
                    let session_manager = session_manager.clone();
                    move |update_type: String,
                          update_author_id: String,
                          value: serde_json::Value| {
                        settings_changed.store(true, Ordering::Relaxed);
                        set_session_handler(
                            session_manager.clone(),
                            update_type,
                            update_author_id,
                            value,
                        )
                    }
                })),
        );

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let log_receiver = log_sender.subscribe();
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    let driver_registration_requests = warp::path!("driver" / String).map({
        |action_string: String| {
            let alvr_dir = get_alvr_dir().unwrap();
            let res = show_err(match action_string.as_str() {
                "register" => driver_registration(&alvr_dir.clone(), true),
                "unregister" => driver_registration(&alvr_dir.clone(), false),
                "unregister-all" => unregister_all_drivers(),
                _ => return reply::with_status(reply(), StatusCode::BAD_REQUEST),
            });
            if res.is_ok() {
                reply::with_status(reply(), StatusCode::OK)
            } else {
                reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    });

    let firewall_rules_requests =
        warp::path!("firewall-rules" / String).map(|action_str: String| {
            let add = action_str == "add";
            let maybe_err = firewall_rules(add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply::json(&maybe_err.unwrap_or(0))
        });

    let audio_devices_request =
        warp::path("audio_devices").map(|| reply::json(&audio::output_audio_devices().ok()));

    let restart_steamvr_request = warp::path("restart_steamvr").map(move || {
        restart_steamvr();
        warp::reply()
    });

    let client_trusted_request = warp::path("trust_client").and(body::json()).and_then({
        let session_manager = session_manager.clone();
        move |ip: IpAddr| {
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            let session_manager = session_manager.clone();
            async move {
                let mut session_manager_ref = session_manager.lock().await;
                let session_ref =
                    &mut *session_manager_ref.get_mut("", SessionUpdateType::ClientList);
                let maybe_client_entry_ref =
                    session_ref.last_clients.iter_mut().find(|c| c.ip == ip);
                if let Some(client_entry_ref) = maybe_client_entry_ref {
                    client_entry_ref.trusted = true;
                }

                if let Err(e) = update_client_listeners_notifier.send(()) {
                    warn!("Failed to notify client listeners restart: {:?}", e);
                };

                Ok::<_, Infallible>(reply::with_status(reply(), StatusCode::OK))
            }
        }
    });

    let version_request = warp::path("version").map(|| ALVR_SERVER_VERSION.to_owned());

    let web_server_port = session_manager
        .lock()
        .await
        .get()
        .to_settings()
        .connection
        .web_server_port;

    let web_server_future = warp::serve(
        index_request
            .or(settings_schema_request)
            .or(session_requests)
            .or(log_subscription)
            .or(driver_registration_requests)
            .or(firewall_rules_requests)
            .or(audio_devices_request)
            .or(restart_steamvr_request)
            .or(client_trusted_request)
            .or(version_request)
            .or(files_requests)
            .with(reply::with::header(
                "Cache-Control",
                "no-cache, no-store, must-revalidate",
            )),
    )
    .run(([0, 0, 0, 0], web_server_port));

    tokio::select! {
        _ = web_server_future => (),
        _ = shutdown_receiver.recv() => ()
    }

    Ok(())
}

async fn client_found_callback(session_manager: Arc<AMutex<SessionManager>>, client_ip: IpAddr) {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let session_manager_ref = &mut session_manager.lock().await;
    let session_desc_ref =
        &mut session_manager_ref.get_mut(SERVER_SESSION_UPDATE_ID, SessionUpdateType::ClientList);

    let maybe_known_client_ref = session_desc_ref
        .last_clients
        .iter_mut()
        .find(|connection_desc| connection_desc.ip == client_ip);

    if let Some(known_client_ref) = maybe_known_client_ref {
        known_client_ref.last_update_ms_since_epoch = now_ms as _;
    } else {
        session_desc_ref.last_clients.push(ClientConnectionDesc {
            trusted: false,
            manually_added: false,
            last_update_ms_since_epoch: now_ms as _,
            ip: client_ip,
            device_name: None,
        });
    }
}

async fn create_control_socket(
    client_ip: IpAddr,
    settings: Settings,
) -> StrResult<ControlSocket<ClientControlPacket, ServerControlPacket>> {
    loop {
        let maybe_control_socket = ControlSocket::connect_to_client(
            client_ip,
            |server_config: ServerConfigPacket, server_ip| {
                let eye_width;
                let eye_height;
                match settings.video.render_resolution {
                    FrameSize::Scale(scale) => {
                        let (native_eye_width, native_eye_height) =
                            server_config.native_eye_resolution;
                        eye_width = native_eye_width as f32 * scale;
                        eye_height = native_eye_height as f32 * scale;
                    }
                    FrameSize::Absolute { width, height } => {
                        eye_width = width as f32 / 2_f32;
                        eye_height = height as f32 / 2_f32;
                    }
                }
                let eye_resolution = (align32(eye_width), align32(eye_height));

                let web_gui_url = format!(
                    "http://{}:{}/",
                    server_ip, settings.connection.web_server_port
                );

                ClientConfigPacket {
                    settings: settings.clone(),
                    eye_resolution,
                    web_gui_url,
                }
            },
        )
        .await;

        match maybe_control_socket {
            Ok(control_socket) => break Ok(control_socket),
            Err(e) => warn!("{}", e),
        }
    }
}

async fn setup_streams(
    settings: Settings,
    control_socket: &ControlSocket<ClientControlPacket, ServerControlPacket>,
) -> StrResult {
    let stream_manager = StreamManager::new(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        settings.connection.stream_socket_config,
    )
    .await?;

    // todo: create input/output streams, bind to C++ callbacks

    Ok(())
}

async fn connection_loop(
    session_manager: Arc<AMutex<SessionManager>>,
    update_client_listeners_notifier: broadcast::Sender<()>,
    mut shutdown_receiver: broadcast::Receiver<()>,
) -> StrResult {
    // Some settings cannot be applied right away because they were used to initialize some key
    // driver components. For these settings, send the cached values to the client.
    let settings_cache = session_manager.lock().await.get().to_settings();

    loop {
        let mut update_client_listeners_receiver = update_client_listeners_notifier.subscribe();

        let client_discovery = async {
            let res = search_client_loop(None, |client_ip| {
                client_found_callback(session_manager.clone(), client_ip)
            })
            .await;

            Err::<(), _>(res.err().unwrap())
        };

        let get_control_sockets = session_manager
            .lock()
            .await
            .get()
            .last_clients
            .iter()
            .map(|client| Box::pin(create_control_socket(client.ip, settings_cache.clone())))
            .collect::<Vec<_>>();

        // launch all futures at once, get the output of the first that completes, cancel all other
        // futures.
        let mut control_socket: ControlSocket<ClientControlPacket, ServerControlPacket> = tokio::select! {
            Err(e) = client_discovery => break trace_str!("Client discovery failed: {}", e),
            (Ok(control_socket), _, _) = future::select_all(get_control_sockets) => {
                control_socket
            }
            _ = update_client_listeners_receiver.recv() => continue,
            _ = shutdown_receiver.recv() => break Ok(()),
            else => unreachable!()
        };

        setup_streams(settings_cache.clone(), &control_socket).await?;

        tokio::select! {
            _ = control_socket.recv() => continue,
            _ = shutdown_receiver.recv() => break Ok(()),
        }
    }
}

pub fn init(log_sender: broadcast::Sender<String>) -> StrResult {
    let alvr_dir = get_alvr_dir()?;

    if let Err(e) = restore_driver_paths_backup() {
        info!(
            "Failed to restore drivers paths backup (usually not an error): {}",
            e
        );
        // This is not fatal. This happens if the user did register ALVR driver through the setup
        // wizard.
    }

    if let Some(runtime) = &*MAYBE_RUNTIME.lock() {
        let session_manager = Arc::new(AMutex::new(SessionManager::new(&alvr_dir)));

        let (shutdown_notifier, shutdown_receiver) = broadcast::channel(1);
        let (update_client_listeners_notifier, _) = broadcast::channel(1);

        runtime.spawn(show_err_async(web_server(
            session_manager.clone(),
            log_sender,
            shutdown_receiver,
            update_client_listeners_notifier.clone(),
        )));
        runtime.spawn(show_err_async(connection_loop(
            session_manager,
            update_client_listeners_notifier,
            shutdown_notifier.subscribe(),
        )));

        *MAYBE_SHUTDOWN_NOTIFIER.lock() = Some(shutdown_notifier);
    }

    let alvr_dir_c_string = CString::new(alvr_dir.to_string_lossy().to_string()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };

    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(|| {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_CAPACITY);
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

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        log(log::Level::Error, string_ptr);
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

    extern "C" fn _shutdown_runtime() {
        shutdown_runtime();
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    ShutdownRuntime = Some(_shutdown_runtime);

    CppEntryPoint(interface_name, return_code)
}
