#![allow(clippy::if_same_then_else)]

use crate::{
    decoder::{self, DecoderConfig, DecoderSink, DecoderSource},
    logging_backend::{LogMirrorData, LOG_CHANNEL_SENDER},
    platform,
    sockets::AnnouncerSocket,
    statistics::StatisticsManager,
    storage::Config,
    ClientCapabilities, ClientCoreEvent,
};
use alvr_audio::AudioDevice;
use alvr_common::{
    debug, error, info,
    parking_lot::{Condvar, Mutex, RwLock},
    wait_rwlock, warn, AnyhowToCon, ConResult, ConnectionError, ConnectionState, LifecycleState,
    ALVR_VERSION,
};
use alvr_packets::{
    ClientConnectionResult, ClientControlPacket, ClientStatistics, Haptics, ServerControlPacket,
    StreamConfigPacket, Tracking, VideoPacketHeader, VideoStreamingCapabilities, ViewParams, AUDIO,
    HAPTICS, STATISTICS, TRACKING, VIDEO,
};
use alvr_session::settings_schema::Switch;
use alvr_sockets::{
    ControlSocketSender, PeerType, ProtoControlSocket, StreamSender, StreamSocketBuilder,
    KEEPALIVE_INTERVAL, KEEPALIVE_TIMEOUT,
};
use std::{
    collections::VecDeque,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "android")]
use crate::audio;
#[cfg(not(target_os = "android"))]
use alvr_audio as audio;

const INITIAL_MESSAGE: &str = concat!(
    "Searching for streamer...\n",
    "Open ALVR on your PC then click \"Trust\"\n",
    "next to the client entry",
);
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the streamer.\nNetwork error.";
const SUCCESS_CONNECT_MESSAGE: &str = "Successful connection!\nPlease wait...";
const LOCAL_TRY_MESSAGE: &str = "Trying to connect to localhost...";
// const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
//     "Streamer and client have\n",
//     "incompatible types.\n",
//     "Please update either the app\n",
//     "on the PC or on the headset",
// );
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The streamer is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The streamer has disconnected.";
const CONNECTION_TIMEOUT_MESSAGE: &str = "Connection timeout.";

const DISCOVERY_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const CONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const HANDSHAKE_ACTION_TIMEOUT: Duration = Duration::from_secs(2);
const STREAMING_RECV_TIMEOUT: Duration = Duration::from_millis(500);

const MAX_UNREAD_PACKETS: usize = 10; // Applies per stream

#[derive(Default)]
pub struct ConnectionContext {
    pub state: RwLock<ConnectionState>,
    pub disconnected_notif: Condvar,
    pub control_sender: Mutex<Option<ControlSocketSender<ClientControlPacket>>>,
    pub tracking_sender: Mutex<Option<StreamSender<Tracking>>>,
    pub statistics_sender: Mutex<Option<StreamSender<ClientStatistics>>>,
    pub statistics_manager: Mutex<Option<StatisticsManager>>,
    pub decoder_sink: Mutex<Option<DecoderSink>>,
    pub decoder_source: Mutex<Option<DecoderSource>>,
    // todo: the server is supposed to receive and send view configs for each frame
    pub view_params_queue: RwLock<VecDeque<(Duration, [ViewParams; 2])>>,
    pub last_good_view_params: RwLock<[ViewParams; 2]>,
}

fn set_hud_message(event_queue: &Mutex<VecDeque<ClientCoreEvent>>, message: &str) {
    let message = format!(
        "ALVR v{}\nhostname: {}\nIP: {}\n\n{message}",
        *ALVR_VERSION,
        Config::load().hostname,
        platform::local_ip(),
    );

    event_queue
        .lock()
        .push_back(ClientCoreEvent::UpdateHudMessage(message));
}

fn is_streaming(ctx: &ConnectionContext) -> bool {
    *ctx.state.read() == ConnectionState::Streaming
}

pub fn connection_lifecycle_loop(
    capabilities: ClientCapabilities,
    ctx: Arc<ConnectionContext>,
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    event_queue: Arc<Mutex<VecDeque<ClientCoreEvent>>>,
) {
    set_hud_message(&event_queue, INITIAL_MESSAGE);

    while *lifecycle_state.read() != LifecycleState::ShuttingDown {
        if *lifecycle_state.read() == LifecycleState::Resumed {
            if let Err(e) = connection_pipeline(
                capabilities.clone(),
                Arc::clone(&ctx),
                Arc::clone(&lifecycle_state),
                Arc::clone(&event_queue),
            ) {
                let message = format!("Connection error:\n{e}\nCheck the PC for more details");
                set_hud_message(&event_queue, &message);
                error!("Connection error: {e}");
            }
        } else {
            debug!("Skip try connection because the device is sleeping");
        }

        *ctx.state.write() = ConnectionState::Disconnected;
        ctx.disconnected_notif.notify_all();

        thread::sleep(CONNECTION_RETRY_INTERVAL);
    }
}

fn connection_pipeline(
    capabilities: ClientCapabilities,
    ctx: Arc<ConnectionContext>,
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    event_queue: Arc<Mutex<VecDeque<ClientCoreEvent>>>,
) -> ConResult {
    let (mut proto_control_socket, server_ip) = {
        let config = Config::load();
        let announcer_socket = AnnouncerSocket::new(&config.hostname).to_con()?;
        let listener_socket =
            alvr_sockets::get_server_listener(HANDSHAKE_ACTION_TIMEOUT).to_con()?;

        loop {
            if *lifecycle_state.write() != LifecycleState::Resumed {
                return Ok(());
            }

            let mut is_broadcast_ok = false;
            if let Err(e) = announcer_socket.announce_broadcast() {
                debug!("Couldn't announce to localhost, retrying on local... {e:}");

                set_hud_message(&event_queue, LOCAL_TRY_MESSAGE);
            } else {
                is_broadcast_ok = true;
            }

            if let Ok(pair) = ProtoControlSocket::connect_to(
                DISCOVERY_RETRY_PAUSE,
                PeerType::Server(&listener_socket),
            ) {
                set_hud_message(&event_queue, SUCCESS_CONNECT_MESSAGE);
                break pair;
            }

            if !is_broadcast_ok {
                warn!("Couldn't announce to network or connect to localhost.");
                set_hud_message(&event_queue, NETWORK_UNREACHABLE_MESSAGE);

                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);

                set_hud_message(&event_queue, INITIAL_MESSAGE);
                return Ok(());
            }
        }
    };

    let mut connection_state_lock = ctx.state.write();
    let disconnect_notif = Arc::new(Condvar::new());

    *connection_state_lock = ConnectionState::Connecting;

    let microphone_sample_rate = AudioDevice::new_input(None)
        .to_con()?
        .input_sample_rate()
        .to_con()?;

    proto_control_socket
        .send(&ClientConnectionResult::ConnectionAccepted {
            client_protocol_id: alvr_common::protocol_id_u64(),
            display_name: platform::platform().to_string(),
            server_ip,
            streaming_capabilities: Some(
                alvr_packets::encode_video_streaming_capabilities(&VideoStreamingCapabilities {
                    default_view_resolution: capabilities.default_view_resolution,
                    supported_refresh_rates: capabilities.refresh_rates,
                    microphone_sample_rate,
                    supports_foveated_encoding: capabilities.foveated_encoding,
                    encoder_high_profile: capabilities.encoder_high_profile,
                    encoder_10_bits: capabilities.encoder_10_bits,
                    encoder_av1: capabilities.encoder_av1,
                })
                .to_con()?,
            ),
        })
        .to_con()?;
    let config_packet =
        proto_control_socket.recv::<StreamConfigPacket>(HANDSHAKE_ACTION_TIMEOUT)?;

    let (settings, negotiated_config) =
        alvr_packets::decode_stream_config(&config_packet).to_con()?;

    let streaming_start_event = ClientCoreEvent::StreamingStarted {
        settings: Box::new(settings.clone()),
        negotiated_config: negotiated_config.clone(),
    };

    *ctx.statistics_manager.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size,
        Duration::from_secs_f32(1.0 / negotiated_config.refresh_rate_hint),
        if let Switch::Enabled(config) = settings.headset.controllers {
            config.steamvr_pipeline_frames
        } else {
            0.0
        },
    ));

    let (mut control_sender, mut control_receiver) = proto_control_socket
        .split(STREAMING_RECV_TIMEOUT)
        .to_con()?;

    match control_receiver.recv(HANDSHAKE_ACTION_TIMEOUT) {
        Ok(ServerControlPacket::StartStream) => {
            info!("Stream starting");
            set_hud_message(&event_queue, STREAM_STARTING_MESSAGE);
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_hud_message(&event_queue, SERVER_RESTART_MESSAGE);
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {e}");
            set_hud_message(&event_queue, SERVER_DISCONNECTED_MESSAGE);
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_hud_message(&event_queue, "Unexpected packet");
            return Ok(());
        }
    }

    let stream_socket_builder = StreamSocketBuilder::listen_for_server(
        Duration::from_secs(1),
        settings.connection.stream_port,
        settings.connection.stream_protocol,
        settings.connection.dscp,
        settings.connection.client_send_buffer_bytes,
        settings.connection.client_recv_buffer_bytes,
    )
    .to_con()?;

    if let Err(e) = control_sender.send(&ClientControlPacket::StreamReady) {
        info!("Server disconnected. Cause: {e:?}");
        set_hud_message(&event_queue, SERVER_DISCONNECTED_MESSAGE);
        return Ok(());
    }

    let mut stream_socket = stream_socket_builder.accept_from_server(
        server_ip,
        settings.connection.stream_port,
        settings.connection.packet_size as _,
        HANDSHAKE_ACTION_TIMEOUT,
    )?;

    info!("Connected to server");

    let mut video_receiver =
        stream_socket.subscribe_to_stream::<VideoPacketHeader>(VIDEO, MAX_UNREAD_PACKETS);
    let mut game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO, MAX_UNREAD_PACKETS);
    let tracking_sender = stream_socket.request_stream(TRACKING);
    let mut haptics_receiver =
        stream_socket.subscribe_to_stream::<Haptics>(HAPTICS, MAX_UNREAD_PACKETS);
    let statistics_sender = stream_socket.request_stream(STATISTICS);

    let video_receive_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let event_queue = Arc::clone(&event_queue);
        move || {
            let mut stream_corrupted = false;
            while is_streaming(&ctx) {
                let data = match video_receiver.recv(STREAMING_RECV_TIMEOUT) {
                    Ok(data) => data,
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(ConnectionError::Other(_)) => return,
                };
                let Ok((header, nal)) = data.get() else {
                    return;
                };

                if let Some(stats) = &mut *ctx.statistics_manager.lock() {
                    stats.report_video_packet_received(header.timestamp);
                }

                if header.is_idr {
                    stream_corrupted = false;
                } else if data.had_packet_loss() {
                    stream_corrupted = true;
                    if let Some(sender) = &mut *ctx.control_sender.lock() {
                        sender.send(&ClientControlPacket::RequestIdr).ok();
                    }
                    warn!("Network dropped video packet");
                }

                if !stream_corrupted || !settings.connection.avoid_video_glitching {
                    if capabilities.external_decoder {
                        let mut view_params = *ctx.last_good_view_params.read();
                        for (timestamp, views) in &*ctx.view_params_queue.read() {
                            if *timestamp == header.timestamp {
                                view_params = *views;
                                break;
                            }
                        }
                        event_queue.lock().push_back(ClientCoreEvent::FrameReady {
                            timestamp: header.timestamp,
                            view_params,
                            nal: nal.to_vec(),
                        });
                    } else if !ctx
                        .decoder_sink
                        .lock()
                        .as_mut()
                        .map(|sink| sink.push_nal(header.timestamp, nal))
                        .unwrap_or(false)
                    {
                        stream_corrupted = true;
                        if let Some(sender) = &mut *ctx.control_sender.lock() {
                            sender.send(&ClientControlPacket::RequestIdr).ok();
                        }
                        warn!("Dropped video packet. Reason: Decoder saturation")
                    }
                } else {
                    if let Some(sender) = &mut *ctx.control_sender.lock() {
                        sender.send(&ClientControlPacket::RequestIdr).ok();
                    }
                    warn!("Dropped video packet. Reason: Waiting for IDR frame")
                }
            }
        }
    });

    let game_audio_thread = if let Switch::Enabled(config) = settings.audio.game_audio {
        let device = AudioDevice::new_output(None, None).to_con()?;
        thread::spawn({
            let ctx = Arc::clone(&ctx);
            move || {
                while is_streaming(&ctx) {
                    alvr_common::show_err(audio::play_audio_loop(
                        || is_streaming(&ctx),
                        &device,
                        2,
                        negotiated_config.game_audio_sample_rate,
                        config.buffering.clone(),
                        &mut game_audio_receiver,
                    ));
                }
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let microphone_thread = if matches!(settings.audio.microphone, Switch::Enabled(_)) {
        let device = AudioDevice::new_input(None).to_con()?;

        let microphone_sender = stream_socket.request_stream(AUDIO);

        thread::spawn({
            let ctx = Arc::clone(&ctx);
            move || {
                while is_streaming(&ctx) {
                    let ctx = Arc::clone(&ctx);
                    match audio::record_audio_blocking(
                        Arc::new(move || is_streaming(&ctx)),
                        microphone_sender.clone(),
                        &device,
                        1,
                        false,
                    ) {
                        Ok(()) => break,
                        Err(e) => {
                            error!("Audio record error: {e}");

                            continue;
                        }
                    }
                }
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let haptics_receive_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let event_queue = Arc::clone(&event_queue);
        move || {
            while is_streaming(&ctx) {
                let data = match haptics_receiver.recv(STREAMING_RECV_TIMEOUT) {
                    Ok(packet) => packet,
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(ConnectionError::Other(_)) => return,
                };
                let Ok(haptics) = data.get_header() else {
                    return;
                };

                event_queue.lock().push_back(ClientCoreEvent::Haptics {
                    device_id: haptics.device_id,
                    duration: haptics.duration,
                    frequency: haptics.frequency,
                    amplitude: haptics.amplitude,
                });
            }
        }
    });

    let (log_channel_sender, log_channel_receiver) = mpsc::channel();

    let control_send_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let event_queue = Arc::clone(&event_queue);
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            let mut keepalive_deadline = Instant::now();

            #[cfg(target_os = "android")]
            let mut battery_deadline = Instant::now();

            while is_streaming(&ctx) && *lifecycle_state.read() == LifecycleState::Resumed {
                if let (Ok(packet), Some(sender)) = (
                    log_channel_receiver.recv_timeout(STREAMING_RECV_TIMEOUT),
                    &mut *ctx.control_sender.lock(),
                ) {
                    if let Err(e) = sender.send(&packet) {
                        info!("Server disconnected. Cause: {e:?}");
                        set_hud_message(&event_queue, SERVER_DISCONNECTED_MESSAGE);

                        break;
                    }
                }

                if Instant::now() > keepalive_deadline {
                    if let Some(sender) = &mut *ctx.control_sender.lock() {
                        sender.send(&ClientControlPacket::KeepAlive).ok();

                        keepalive_deadline = Instant::now() + KEEPALIVE_INTERVAL;
                    }
                }

                #[cfg(target_os = "android")]
                if Instant::now() > battery_deadline {
                    let (gauge_value, is_plugged) = platform::get_battery_status();
                    if let Some(sender) = &mut *ctx.control_sender.lock() {
                        sender
                            .send(&ClientControlPacket::Battery(crate::BatteryPacket {
                                device_id: *alvr_common::HEAD_ID,
                                gauge_value,
                                is_plugged,
                            }))
                            .ok();
                    }

                    battery_deadline = Instant::now() + Duration::from_secs(5);
                }
            }

            disconnect_notif.notify_one();
        }
    });

    let control_receive_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let event_queue = Arc::clone(&event_queue);
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            let mut disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            while is_streaming(&ctx) {
                let maybe_packet = control_receiver.recv(STREAMING_RECV_TIMEOUT);

                match maybe_packet {
                    Ok(ServerControlPacket::DecoderConfig(config)) => {
                        if capabilities.external_decoder {
                            event_queue
                                .lock()
                                .push_back(ClientCoreEvent::DecoderConfig {
                                    codec: config.codec,
                                    config_nal: config.config_buffer,
                                });
                        } else if ctx.decoder_sink.lock().is_none() {
                            let config = DecoderConfig {
                                codec: config.codec,
                                force_software_decoder: settings.video.force_software_decoder,
                                max_buffering_frames: settings.video.max_buffering_frames,
                                buffering_history_weight: settings.video.buffering_history_weight,
                                options: settings.video.mediacodec_extra_options.clone(),
                                config_buffer: config.config_buffer,
                            };

                            let (sink, source) = decoder::create_decoder(config, {
                                let ctx = Arc::clone(&ctx);
                                move |target_timestamp| {
                                    if let Some(stats) = &mut *ctx.statistics_manager.lock() {
                                        stats.report_frame_decoded(target_timestamp);
                                    }
                                }
                            });

                            *ctx.decoder_sink.lock() = Some(sink);
                            *ctx.decoder_source.lock() = Some(source);

                            if let Some(sender) = &mut *ctx.control_sender.lock() {
                                sender.send(&ClientControlPacket::RequestIdr).ok();
                            }
                        }
                    }
                    Ok(ServerControlPacket::Restarting) => {
                        info!("{SERVER_RESTART_MESSAGE}");
                        set_hud_message(&event_queue, SERVER_RESTART_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                    Ok(_) => (),
                    Err(ConnectionError::TryAgain(_)) => {
                        if Instant::now() > disconnection_deadline {
                            info!("{CONNECTION_TIMEOUT_MESSAGE}");
                            set_hud_message(&event_queue, CONNECTION_TIMEOUT_MESSAGE);
                            disconnect_notif.notify_one();
                        } else {
                            continue;
                        }
                    }
                    Err(e) => {
                        info!("{SERVER_DISCONNECTED_MESSAGE} Cause: {e}");
                        set_hud_message(&event_queue, SERVER_DISCONNECTED_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                }

                disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            }
        }
    });

    let stream_receive_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let event_queue = Arc::clone(&event_queue);
        let disconnect_notif = Arc::clone(&disconnect_notif);
        move || {
            while is_streaming(&ctx) {
                match stream_socket.recv() {
                    Ok(()) => (),
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(e) => {
                        info!("Client disconnected. Cause: {e}");
                        set_hud_message(&event_queue, SERVER_DISCONNECTED_MESSAGE);
                        disconnect_notif.notify_one();
                    }
                }
            }
        }
    });

    *ctx.control_sender.lock() = Some(control_sender);
    *ctx.tracking_sender.lock() = Some(tracking_sender);
    *ctx.statistics_sender.lock() = Some(statistics_sender);
    if let Switch::Enabled(filter_level) = settings.logging.client_log_report_level {
        *LOG_CHANNEL_SENDER.lock() = Some(LogMirrorData {
            sender: log_channel_sender,
            filter_level,
        });
    }
    event_queue.lock().push_back(streaming_start_event);

    *connection_state_lock = ConnectionState::Streaming;

    // Make sure IPD and FoV are resent after reconnection
    // todo: send this data as part of the connection handshake
    ctx.view_params_queue.write().clear();

    // Unlock CONNECTION_STATE and block thread
    wait_rwlock(&disconnect_notif, &mut connection_state_lock);

    *connection_state_lock = ConnectionState::Disconnecting;

    *ctx.control_sender.lock() = None;
    *ctx.tracking_sender.lock() = None;
    *ctx.statistics_sender.lock() = None;
    *LOG_CHANNEL_SENDER.lock() = None;

    event_queue
        .lock()
        .push_back(ClientCoreEvent::StreamingStopped);

    *ctx.decoder_sink.lock() = None;
    *ctx.decoder_source.lock() = None;

    // Remove lock to allow threads to properly exit:
    drop(connection_state_lock);

    video_receive_thread.join().ok();
    game_audio_thread.join().ok();
    microphone_thread.join().ok();
    haptics_receive_thread.join().ok();
    control_send_thread.join().ok();
    control_receive_thread.join().ok();
    stream_receive_thread.join().ok();

    Ok(())
}
