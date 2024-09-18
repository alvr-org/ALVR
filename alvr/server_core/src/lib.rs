mod bitrate;
mod body_tracking;
mod c_api;
mod connection;
mod face_tracking;
mod hand_gestures;
mod haptics;
mod input_mapping;
mod logging_backend;
mod sockets;
mod statistics;
mod tracking;
mod web_server;

pub use c_api::*;
pub use logging_backend::init_logging;

use crate::connection::VideoPacket;
use alvr_common::{
    dbg_server_core, error,
    glam::Vec2,
    once_cell::sync::Lazy,
    parking_lot::{Mutex, RwLock},
    settings_schema::Switch,
    warn, ConnectionState, Fov, LifecycleState, Pose, RelaxedAtomic, DEVICE_ID_TO_PATH,
};
use alvr_events::{EventType, HapticsEvent};
use alvr_filesystem as afs;
use alvr_packets::{
    BatteryInfo, ButtonEntry, ClientListAction, DecoderInitializationConfig, Haptics, Tracking,
    VideoPacketHeader,
};
use alvr_server_io::ServerSessionManager;
use alvr_session::{CodecType, OpenvrProperty, Settings};
use alvr_sockets::StreamSender;
use bitrate::{BitrateManager, DynamicEncoderParams};
use statistics::StatisticsManager;
use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fs::File,
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind};
use tokio::{runtime::Runtime, sync::broadcast};

static FILESYSTEM_LAYOUT: OnceLock<afs::Layout> = OnceLock::new();

pub fn initialize_environment(layout: afs::Layout) {
    FILESYSTEM_LAYOUT.set(layout).unwrap();
}

// This is lazily initialized when initializing logging or ServerCoreContext. So FILESYSTEM_LAYOUT
// needs to be initialized first using initialize_environment().
// NB: this must remain a global because only one instance should exist for the whole application
// execution time.
static SESSION_MANAGER: Lazy<RwLock<ServerSessionManager>> = Lazy::new(|| {
    RwLock::new(ServerSessionManager::new(
        FILESYSTEM_LAYOUT.get().map(|l| l.session()),
    ))
});

// todo: use this as the network packet
pub struct ViewsConfig {
    // transforms relative to the head
    pub local_view_transforms: [Pose; 2],
    pub fov: [Fov; 2],
}

pub enum ServerCoreEvent {
    SetOpenvrProperty {
        device_id: u64,
        prop: OpenvrProperty,
    },
    ClientConnected,
    ClientDisconnected,
    Battery(BatteryInfo),
    PlayspaceSync(Vec2),
    ViewsConfig(ViewsConfig),
    Tracking {
        tracking: Box<Tracking>,
        controllers_pose_time_offset: Duration,
    },
    Buttons(Vec<ButtonEntry>), // Note: this is after mapping
    RequestIDR,
    CaptureFrame,
    GameRenderLatencyFeedback(Duration), // only used for SteamVR
    ShutdownPending,
    RestartPending,
}

pub struct ConnectionContext {
    events_sender: mpsc::Sender<ServerCoreEvent>,
    statistics_manager: Mutex<Option<StatisticsManager>>,
    bitrate_manager: Mutex<BitrateManager>,
    decoder_config: Mutex<Option<DecoderInitializationConfig>>,
    video_mirror_sender: Mutex<Option<broadcast::Sender<Vec<u8>>>>,
    video_recording_file: Mutex<Option<File>>,
    connection_threads: Mutex<Vec<JoinHandle<()>>>,
    clients_to_be_removed: Mutex<HashSet<String>>,
    video_channel_sender: Mutex<Option<SyncSender<VideoPacket>>>,
    haptics_sender: Mutex<Option<StreamSender<Haptics>>>,
}

pub fn create_recording_file(connection_context: &ConnectionContext, settings: &Settings) {
    let codec = settings.video.preferred_codec;
    let ext = match codec {
        CodecType::H264 => "h264",
        CodecType::Hevc => "h265",
        CodecType::AV1 => "av1",
    };

    let path = FILESYSTEM_LAYOUT.get().unwrap().log_dir.join(format!(
        "recording.{}.{ext}",
        chrono::Local::now().format("%F.%H-%M-%S")
    ));

    match File::create(path) {
        Ok(mut file) => {
            if let Some(config) = &*connection_context.decoder_config.lock() {
                file.write_all(&config.config_buffer).ok();
            }

            *connection_context.video_recording_file.lock() = Some(file);

            connection_context
                .events_sender
                .send(ServerCoreEvent::RequestIDR)
                .ok();
        }
        Err(e) => {
            error!("Failed to record video on disk: {e}");
        }
    }
}

pub fn notify_restart_driver() {
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes(ProcessesToUpdate::All);

    if system
        .processes_by_name(OsStr::new(&afs::dashboard_fname()))
        .next()
        .is_some()
    {
        alvr_events::send_event(EventType::ServerRequestsSelfRestart);
    } else {
        error!("Cannot restart SteamVR. No dashboard process found on local device.");
    }
}

pub fn settings() -> Settings {
    SESSION_MANAGER.read().settings().clone()
}

pub fn registered_button_set() -> HashSet<u64> {
    let session_manager = SESSION_MANAGER.read();
    if let Switch::Enabled(input_mapping) = &session_manager.settings().headset.controllers {
        input_mapping::registered_button_set(&input_mapping.emulation_mode)
    } else {
        HashSet::new()
    }
}

pub struct ServerCoreContext {
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    is_restarting: RelaxedAtomic,
    connection_context: Arc<ConnectionContext>,
    connection_thread: Arc<RwLock<Option<JoinHandle<()>>>>,
    webserver_runtime: Option<Runtime>,
}

impl ServerCoreContext {
    pub fn new() -> (Self, mpsc::Receiver<ServerCoreEvent>) {
        dbg_server_core!("Creating");

        if SESSION_MANAGER
            .read()
            .settings()
            .extra
            .logging
            .prefer_backtrace
        {
            env::set_var("RUST_BACKTRACE", "1");
        }

        SESSION_MANAGER.write().clean_client_list();

        let (events_sender, events_receiver) = mpsc::channel();

        let connection_context = Arc::new(ConnectionContext {
            events_sender,
            statistics_manager: Mutex::new(None),
            bitrate_manager: Mutex::new(BitrateManager::new(256, 60.0)),
            decoder_config: Mutex::new(None),
            video_mirror_sender: Mutex::new(None),
            video_recording_file: Mutex::new(None),
            connection_threads: Mutex::new(Vec::new()),
            clients_to_be_removed: Mutex::new(HashSet::new()),
            video_channel_sender: Mutex::new(None),
            haptics_sender: Mutex::new(None),
        });

        let webserver_runtime = Runtime::new().unwrap();
        webserver_runtime.spawn({
            let connection_context = Arc::clone(&connection_context);
            async move { alvr_common::show_err(web_server::web_server(connection_context).await) }
        });

        (
            Self {
                lifecycle_state: Arc::new(RwLock::new(LifecycleState::StartingUp)),
                is_restarting: RelaxedAtomic::new(false),
                connection_context,

                connection_thread: Arc::new(RwLock::new(None)),
                webserver_runtime: Some(webserver_runtime),
            },
            events_receiver,
        )
    }

    pub fn start_connection(&self) {
        dbg_server_core!("start_connection");

        // Note: Idle state is not used on the server side
        *self.lifecycle_state.write() = LifecycleState::Resumed;

        let connection_context = Arc::clone(&self.connection_context);
        let lifecycle_state = Arc::clone(&self.lifecycle_state);
        *self.connection_thread.write() = Some(thread::spawn(move || {
            connection::handshake_loop(connection_context, lifecycle_state);
        }));
    }

    pub fn send_haptics(&self, haptics: Haptics) {
        dbg_server_core!("send_haptics");

        let haptics_config = {
            let session_manager_lock = SESSION_MANAGER.read();

            if session_manager_lock.settings().extra.logging.log_haptics {
                alvr_events::send_event(EventType::Haptics(HapticsEvent {
                    path: DEVICE_ID_TO_PATH
                        .get(&haptics.device_id)
                        .map(|p| (*p).to_owned())
                        .unwrap_or_else(|| format!("Unknown (ID: {:#16x})", haptics.device_id)),
                    duration: haptics.duration,
                    frequency: haptics.frequency,
                    amplitude: haptics.amplitude,
                }))
            }

            session_manager_lock
                .settings()
                .headset
                .controllers
                .as_option()
                .and_then(|c| c.haptics.as_option().cloned())
        };

        if let (Some(config), Some(sender)) = (
            haptics_config,
            &mut *self.connection_context.haptics_sender.lock(),
        ) {
            sender
                .send_header(&haptics::map_haptics(&config, haptics))
                .ok();
        }
    }

    pub fn set_video_config_nals(&self, config_buffer: Vec<u8>, codec: CodecType) {
        dbg_server_core!("set_video_config_nals");

        if let Some(sender) = &*self.connection_context.video_mirror_sender.lock() {
            sender.send(config_buffer.clone()).ok();
        }

        if let Some(file) = &mut *self.connection_context.video_recording_file.lock() {
            file.write_all(&config_buffer).ok();
        }

        *self.connection_context.decoder_config.lock() = Some(DecoderInitializationConfig {
            codec,
            config_buffer,
        });
    }

    pub fn send_video_nal(&self, target_timestamp: Duration, nal_buffer: Vec<u8>, is_idr: bool) {
        dbg_server_core!("send_video_nal");

        // start in the corrupts state, the client didn't receive the initial IDR yet.
        static STREAM_CORRUPTED: AtomicBool = AtomicBool::new(true);
        static LAST_IDR_INSTANT: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));

        if let Some(sender) = &*self.connection_context.video_channel_sender.lock() {
            let buffer_size = nal_buffer.len();

            if is_idr {
                STREAM_CORRUPTED.store(false, Ordering::SeqCst);
            }

            if let Switch::Enabled(config) = &SESSION_MANAGER
                .read()
                .settings()
                .extra
                .capture
                .rolling_video_files
            {
                if Instant::now()
                    > *LAST_IDR_INSTANT.lock() + Duration::from_secs(config.duration_s)
                {
                    self.connection_context
                        .events_sender
                        .send(ServerCoreEvent::RequestIDR)
                        .ok();

                    if is_idr {
                        create_recording_file(
                            &self.connection_context,
                            SESSION_MANAGER.read().settings(),
                        );
                        *LAST_IDR_INSTANT.lock() = Instant::now();
                    }
                }
            }

            if !STREAM_CORRUPTED.load(Ordering::SeqCst)
                || !SESSION_MANAGER
                    .read()
                    .settings()
                    .connection
                    .avoid_video_glitching
            {
                if let Some(sender) = &*self.connection_context.video_mirror_sender.lock() {
                    sender.send(nal_buffer.clone()).ok();
                }

                if let Some(file) = &mut *self.connection_context.video_recording_file.lock() {
                    file.write_all(&nal_buffer).ok();
                }

                if matches!(
                    sender.try_send(VideoPacket {
                        header: VideoPacketHeader {
                            timestamp: target_timestamp,
                            is_idr
                        },
                        payload: nal_buffer,
                    }),
                    Err(TrySendError::Full(_))
                ) {
                    STREAM_CORRUPTED.store(true, Ordering::SeqCst);
                    self.connection_context
                        .events_sender
                        .send(ServerCoreEvent::RequestIDR)
                        .ok();
                    warn!("Dropping video packet. Reason: Can't push to network");
                }
            } else {
                warn!("Dropping video packet. Reason: Waiting for IDR frame");
            }

            if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
                let encoder_latency = stats.report_frame_encoded(target_timestamp, buffer_size);

                self.connection_context
                    .bitrate_manager
                    .lock()
                    .report_frame_encoded(target_timestamp, encoder_latency, buffer_size);
            }
        }
    }

    pub fn get_dynamic_encoder_params(&self) -> Option<DynamicEncoderParams> {
        dbg_server_core!("get_dynamic_encoder_params");

        let pair = {
            let session_manager_lock = SESSION_MANAGER.read();
            self.connection_context
                .bitrate_manager
                .lock()
                .get_encoder_params(&session_manager_lock.settings().video.bitrate)
        };

        if let Some((params, stats)) = pair {
            if let Some(stats_manager) = &mut *self.connection_context.statistics_manager.lock() {
                stats_manager.report_nominal_bitrate_stats(stats);
            }

            Some(params)
        } else {
            None
        }
    }

    pub fn report_composed(&self, target_timestamp: Duration, offset: Duration) {
        dbg_server_core!("report_composed");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_frame_composed(target_timestamp, offset);
        }
    }

    pub fn report_present(&self, target_timestamp: Duration, offset: Duration) {
        dbg_server_core!("report_present");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_frame_present(target_timestamp, offset);
        }

        let session_manager_lock = SESSION_MANAGER.read();
        self.connection_context
            .bitrate_manager
            .lock()
            .report_frame_present(
                &session_manager_lock
                    .settings()
                    .video
                    .bitrate
                    .adapt_to_framerate,
            );
    }

    pub fn duration_until_next_vsync(&self) -> Option<Duration> {
        dbg_server_core!("duration_until_next_vsync");

        self.connection_context
            .statistics_manager
            .lock()
            .as_mut()
            .map(|stats| stats.duration_until_next_vsync())
    }

    pub fn restart(self) {
        dbg_server_core!("restart");

        self.is_restarting.set(true);

        // drop is called here for self
    }
}

impl Drop for ServerCoreContext {
    fn drop(&mut self) {
        dbg_server_core!("Drop");

        // Invoke connection runtimes shutdown
        *self.lifecycle_state.write() = LifecycleState::ShuttingDown;

        dbg_server_core!("Setting clients as Disconnecting");
        {
            let mut session_manager_lock = SESSION_MANAGER.write();

            let hostnames = session_manager_lock
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
                session_manager_lock.update_client_list(
                    hostname,
                    ClientListAction::SetConnectionState(ConnectionState::Disconnecting),
                );
            }
        }

        dbg_server_core!("Joining connection thread");
        if let Some(thread) = self.connection_thread.write().take() {
            thread.join().ok();
        }

        // apply openvr config for the next launch
        dbg_server_core!("Setting restart settings chache");
        {
            let mut session_manager_lock = SESSION_MANAGER.write();
            session_manager_lock.session_mut().openvr_config =
                connection::contruct_openvr_config(session_manager_lock.session());
        }

        dbg_server_core!("Restore drivers registration backup");
        if let Some(backup) = SESSION_MANAGER.write().session_mut().drivers_backup.take() {
            alvr_server_io::driver_registration(&backup.other_paths, true).ok();
            alvr_server_io::driver_registration(&[backup.alvr_path], false).ok();
        }

        // todo: check if this is still needed
        while SESSION_MANAGER
            .read()
            .client_list()
            .iter()
            .any(|(_, info)| info.connection_state != ConnectionState::Disconnected)
        {
            thread::sleep(Duration::from_millis(100));
        }

        // Dropping the webserver runtime is bugged on linux and will prevent StemVR shutdown
        if !cfg!(target_os = "linux") {
            self.webserver_runtime.take();
        }
    }
}
