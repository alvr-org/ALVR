#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals)]

mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, process::*, sockets::*, *};
use futures::SinkExt;
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
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
        // much as possible before the process is killed because of alvr_server_bootstrap timeout.
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

        // todo: launch alvr_server_bootstrap with "restart" flag
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

enum ClientListAction {
    AddIfMissing { ip: IpAddr, certificate_pem: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

async fn update_client_list(
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
        &mut session_manager_ref.get_mut(SERVER_SESSION_UPDATE_ID, SessionUpdateType::ClientList);

    let maybe_client_entry = session_desc_ref.last_clients.entry(hostname);

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
            }
        },
        ClientListAction::TrustAndMaybeAddIp(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                let client_connection_ref = entry.get_mut();
                client_connection_ref.trusted = true;
                if let Some(ip) = maybe_ip {
                    client_connection_ref.manual_ips.insert(ip);
                }
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
            }
        }
    }

    if let Err(e) = update_client_listeners_notifier.send(()) {
        warn!("Failed to notify client list update: {:?}", e);
    }
}

async fn web_server(
    session_manager: Arc<AMutex<SessionManager>>,
    log_sender: broadcast::Sender<String>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    let settings_changed = Arc::new(AtomicBool::new(false));

    let web_gui_dir = PathBuf::from(WEB_GUI_DIR_STR);
    let index_request = warp::path::end().and(wfs::file(web_gui_dir.join("index.html")));
    let files_requests = wfs::dir(web_gui_dir);

    let settings_schema_request = warp::path("settings-schema")
        .map(|| reply::json(&settings_schema(settings_cache_default())));

    let get_session_request = warp::get().and(warp::path("session")).and_then({
        let session_manager = session_manager.clone();
        move || {
            let session_manager = session_manager.clone();
            async move { Ok::<_, Infallible>(reply::json(session_manager.lock().await.get())) }
        }
    });
    let post_session_request = warp::post()
        .and(warp::path!("session" / String / String))
        .and(warp::post())
        .and(body::json())
        .and_then({
            let session_manager = session_manager.clone();
            move |update_type: String, update_author_id: String, value: serde_json::Value| {
                settings_changed.store(true, Ordering::Relaxed);
                set_session_handler(
                    session_manager.clone(),
                    update_type,
                    update_author_id,
                    value,
                )
            }
        });

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let log_receiver = log_sender.subscribe();
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    let register_driver_request = warp::path("driver/register").map(|| {
        if driver_registration(&get_alvr_dir().unwrap(), true).is_ok() {
            reply::with_status(reply(), StatusCode::OK)
        } else {
            reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
        }
    });
    let unregister_driver_request =
        warp::path("driver/unregister")
            .and(body::json())
            .map(|path: PathBuf| {
                if driver_registration(&path, false).is_ok() {
                    reply::with_status(reply(), StatusCode::OK)
                } else {
                    reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
                }
            });
    let list_drivers_request = warp::path("driver/list").map(|| {
        if let Ok(list) = get_registered_drivers() {
            reply::json(&list)
        } else {
            reply::json(&Vec::<PathBuf>::new())
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

    let trust_client_request = warp::path("clients/trust").and(body::json()).and_then({
        let session_manager = session_manager.clone();
        let update_client_listeners_notifier = update_client_listeners_notifier.clone();
        move |(hostname, maybe_ip): (String, Option<IpAddr>)| {
            let session_manager = session_manager.clone();
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            async move {
                update_client_list(
                    session_manager,
                    hostname,
                    ClientListAction::TrustAndMaybeAddIp(maybe_ip),
                    update_client_listeners_notifier,
                )
                .await;

                Ok::<_, Infallible>(reply::with_status(reply(), StatusCode::OK))
            }
        }
    });
    let remove_client_request = warp::path("clients/remove").and(body::json()).and_then({
        let session_manager = session_manager.clone();
        move |(hostname, maybe_ip): (String, Option<IpAddr>)| {
            let session_manager = session_manager.clone();
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            async move {
                update_client_list(
                    session_manager,
                    hostname,
                    ClientListAction::RemoveIpOrEntry(maybe_ip),
                    update_client_listeners_notifier,
                )
                .await;

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

    warp::serve(
        index_request
            .or(settings_schema_request)
            .or(get_session_request)
            .or(post_session_request)
            .or(log_subscription)
            .or(register_driver_request)
            .or(unregister_driver_request)
            .or(list_drivers_request)
            .or(firewall_rules_requests)
            .or(audio_devices_request)
            .or(restart_steamvr_request)
            .or(trust_client_request)
            .or(remove_client_request)
            .or(version_request)
            .or(files_requests)
            .with(reply::with::header(
                "Cache-Control",
                "no-cache, no-store, must-revalidate",
            )),
    )
    .run(([0, 0, 0, 0], web_server_port))
    .await;

    trace_str!("Web server closed unexpectedly")
}

async fn create_control_socket(
    clients_data: HashMap<IpAddr, Identity>,
    settings: Settings,
) -> (
    Identity,
    ControlSocket<ClientControlPacket, ServerControlPacket>,
) {
    loop {
        let maybe_control_socket = ControlSocket::connect_to_client(
            &clients_data.keys().cloned().collect::<Vec<_>>(),
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
            Ok(control_socket) => {
                let identity = clients_data.get(&control_socket.peer_ip()).unwrap().clone();
                break (identity, control_socket);
            }
            Err(e) => warn!("{}", e),
        }
    }
}

async fn setup_streams(
    settings: Settings,
    client_identity: Identity,
    control_socket: &ControlSocket<ClientControlPacket, ServerControlPacket>,
) -> StrResult {
    let stream_manager = StreamManager::connect_to_client(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        client_identity,
        settings.connection.stream_socket_config,
    )
    .await?;

    // todo: create input/output streams, bind to C++ callbacks

    Ok(())
}

async fn connection_loop(
    session_manager: Arc<AMutex<SessionManager>>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    // Some settings cannot be applied right away because they were used to initialize some key
    // driver components. For these settings, send the cached values to the client.
    let settings_cache = session_manager.lock().await.get().to_settings();

    loop {
        let mut update_client_listeners_receiver = update_client_listeners_notifier.subscribe();

        let client_discovery = {
            let session_manager = session_manager.clone();
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            async move {
                let res = search_client_loop(None, {
                    |client_ip, client_identity| {
                        update_client_list(
                            session_manager.clone(),
                            client_identity.hostname,
                            ClientListAction::AddIfMissing {
                                ip: client_ip,
                                certificate_pem: client_identity.certificate_pem,
                            },
                            update_client_listeners_notifier.clone(),
                        )
                    }
                })
                .await;

                Err::<(), _>(res.err().unwrap())
            }
        };

        let clients_data = session_manager.lock().await.get().last_clients.iter().fold(
            HashMap::new(),
            |mut clients_data, (hostname, client)| {
                let id = Identity {
                    hostname: hostname.clone(),
                    certificate_pem: client.certificate_pem.clone(),
                };
                clients_data.extend(client.manual_ips.iter().map(|&ip| (ip, id.clone())));
                clients_data.insert(client.last_ip, id);
                clients_data
            },
        );
        let get_control_socket = create_control_socket(clients_data, settings_cache.clone());

        let (identity, mut control_socket) = tokio::select! {
            Err(e) = client_discovery => break trace_str!("Client discovery failed: {}", e),
            pair = get_control_socket => pair,
            _ = update_client_listeners_receiver.recv() => continue,
        };

        if let Err(e) = setup_streams(settings_cache.clone(), identity, &control_socket).await {
            warn!("Setup streams failed: {}", e);
            continue;
        };

        control_socket.recv().await.ok();
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

        let (shutdown_notifier, mut shutdown_receiver) = broadcast::channel(1);
        let (update_client_listeners_notifier, _) = broadcast::channel(1);

        runtime.spawn({
            async move {
                let web_server = show_err_async(web_server(
                    session_manager.clone(),
                    log_sender,
                    update_client_listeners_notifier.clone(),
                ));

                let connection_loop = show_err_async(connection_loop(
                    session_manager,
                    update_client_listeners_notifier,
                ));

                tokio::select! {
                    _ = web_server => (),
                    _ = connection_loop => (),
                    _ = shutdown_receiver.recv() => (),
                }
            }
        });

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
