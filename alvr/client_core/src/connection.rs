#![allow(clippy::if_same_then_else)]

use crate::{
    decoder::{self, DECODER_INIT_CONFIG},
    platform,
    sockets::AnnouncerSocket,
    statistics::StatisticsManager,
    storage::Config,
    ClientCoreEvent, CONTROL_CHANNEL_SENDER, DISCONNECT_NOTIFIER, EVENT_QUEUE, IS_ALIVE,
    IS_RESUMED, IS_STREAMING, STATISTICS_MANAGER, STATISTICS_SENDER, TRACKING_SENDER,
};
use alvr_audio::AudioDevice;
use alvr_common::{glam::UVec2, prelude::*, ALVR_VERSION, HEAD_ID};
use alvr_session::{settings_schema::Switch, SessionDesc};
use alvr_sockets::{
    spawn_cancelable, BatteryPacket, ClientConnectionResult, ClientControlPacket, Haptics,
    PeerType, ProtoControlSocket, ReceiverBuffer, ServerControlPacket, StreamConfigPacket,
    StreamSocketBuilder, VideoStreamingCapabilities, AUDIO, HAPTICS, STATISTICS, TRACKING, VIDEO,
};
use futures::future::BoxFuture;
use serde_json as json;
use std::{
    future,
    net::IpAddr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc as tmpsc, Mutex},
    time,
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
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
// const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
//     "Streamer and client have\n",
//     "incompatible types.\n",
//     "Please update either the app\n",
//     "on the PC or on the headset",
// );
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The streamer is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The streamer has disconnected.";

const DISCOVERY_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const BATTERY_POLL_INTERVAL: Duration = Duration::from_secs(60);

fn set_hud_message(message: &str) {
    let message = format!(
        "ALVR v{}\nhostname: {}\nIP: {}\n\n{message}",
        *ALVR_VERSION,
        Config::load().hostname,
        platform::local_ip(),
    );

    EVENT_QUEUE
        .lock()
        .push_back(ClientCoreEvent::UpdateHudMessage(message));
}

pub fn connection_lifecycle_loop(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
) -> IntResult {
    set_hud_message(INITIAL_MESSAGE);

    let decoder_guard = Arc::new(Mutex::new(()));

    loop {
        check_interrupt!(IS_ALIVE.value());

        if IS_RESUMED.value() {
            if let Err(e) = connection_pipeline(
                recommended_view_resolution,
                supported_refresh_rates.clone(),
                Arc::clone(&decoder_guard),
            ) {
                match e {
                    InterruptibleError::Interrupted => return Ok(()),
                    InterruptibleError::Other(_) => {
                        let message =
                            format!("Connection error:\n{e}\nCheck the PC for more details");
                        error!("{message}");
                        set_hud_message(&message);
                    }
                }
            }
        }

        thread::sleep(CONNECTION_RETRY_INTERVAL);
    }
}

fn connection_pipeline(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
    decoder_guard: Arc<Mutex<()>>,
) -> IntResult {
    let runtime = Runtime::new().map_err(to_int_e!())?;

    let (mut proto_control_socket, server_ip) = {
        let config = Config::load();
        let announcer_socket = AnnouncerSocket::new(&config.hostname).map_err(to_int_e!())?;
        let listener_socket = runtime
            .block_on(alvr_sockets::get_server_listener())
            .map_err(to_int_e!())?;

        loop {
            check_interrupt!(IS_ALIVE.value());

            if let Err(e) = announcer_socket.broadcast() {
                warn!("Broadcast error: {e}");

                set_hud_message(NETWORK_UNREACHABLE_MESSAGE);

                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);

                set_hud_message(INITIAL_MESSAGE);

                return Ok(());
            }

            let maybe_pair = runtime.block_on(async {
                tokio::select! {
                    maybe_pair = ProtoControlSocket::connect_to(PeerType::Server(&listener_socket)) => {
                        maybe_pair.map_err(to_int_e!())
                    },
                    _ = time::sleep(DISCOVERY_RETRY_PAUSE) => Err(InterruptibleError::Interrupted)
                }
            });

            if let Ok(pair) = maybe_pair {
                break pair;
            }
        }
    };

    let microphone_sample_rate = AudioDevice::new_input(None)
        .unwrap()
        .input_sample_rate()
        .unwrap();

    runtime
        .block_on(
            proto_control_socket.send(&ClientConnectionResult::ConnectionAccepted {
                display_name: platform::device_model(),
                server_ip,
                streaming_capabilities: Some(VideoStreamingCapabilities {
                    default_view_resolution: recommended_view_resolution,
                    supported_refresh_rates,
                    microphone_sample_rate,
                }),
            }),
        )
        .map_err(to_int_e!())?;
    let config_packet = runtime
        .block_on(proto_control_socket.recv::<StreamConfigPacket>())
        .map_err(to_int_e!())?;

    runtime
        .block_on(stream_pipeline(
            proto_control_socket,
            config_packet,
            server_ip,
            decoder_guard,
        ))
        .map_err(to_int_e!())
}

async fn stream_pipeline(
    proto_socket: ProtoControlSocket,
    stream_config: StreamConfigPacket,
    server_ip: IpAddr,
    decoder_guard: Arc<Mutex<()>>,
) -> StrResult {
    let (control_sender, mut control_receiver) = proto_socket.split();
    let control_sender = Arc::new(Mutex::new(control_sender));

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => {
            info!("Stream starting");
            set_hud_message(STREAM_STARTING_MESSAGE);
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_hud_message(SERVER_RESTART_MESSAGE);
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {e}");
            set_hud_message(SERVER_DISCONNECTED_MESSAGE);
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_hud_message("Unexpected packet");
            return Ok(());
        }
    }

    let settings = {
        let mut session_desc = SessionDesc::default();
        session_desc
            .merge_from_json(&json::from_str(&stream_config.session_desc).map_err(err!())?)?;
        session_desc.to_settings()
    };

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size as _,
    ));

    let stream_socket_builder = StreamSocketBuilder::listen_for_server(
        settings.connection.stream_port,
        settings.connection.stream_protocol,
        settings.connection.client_send_buffer_bytes,
        settings.connection.client_recv_buffer_bytes,
    )
    .await?;

    if let Err(e) = control_sender
        .lock()
        .await
        .send(&ClientControlPacket::StreamReady)
        .await
    {
        info!("Server disconnected. Cause: {e}");
        set_hud_message(SERVER_DISCONNECTED_MESSAGE);
        return Ok(());
    }

    let stream_socket = tokio::select! {
        res = stream_socket_builder.accept_from_server(
            server_ip,
            settings.connection.stream_port,
            settings.connection.packet_size as _
        ) => res?,
        _ = time::sleep(Duration::from_secs(5)) => {
            return fmt_e!("Timeout while setting up streams");
        }
    };
    let stream_socket = Arc::new(stream_socket);

    info!("Connected to server");

    // create this before initializing the stream on cpp side
    let (control_channel_sender, mut control_channel_receiver) = tmpsc::unbounded_channel();
    *CONTROL_CHANNEL_SENDER.lock() = Some(control_channel_sender);

    {
        let config = &mut *DECODER_INIT_CONFIG.lock();

        config.codec = settings.video.codec;
        config.max_buffering_frames = settings.video.max_buffering_frames;
        config.buffering_history_weight = settings.video.buffering_history_weight;
        config.options = settings
            .video
            .advanced_codec_options
            .mediacodec_extra_options;
    }

    let tracking_send_loop = {
        let mut socket_sender = stream_socket.request_stream(TRACKING).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *TRACKING_SENDER.lock() = Some(data_sender);

            while let Some(tracking) = data_receiver.recv().await {
                socket_sender.send(&tracking, vec![]).await.ok();

                // Note: this is not the best place to report the acquired input. Instead it should
                // be done as soon as possible (or even just before polling the input). Instead this
                // is reported late to partially compensate for lack of network latency measurement,
                // so the server can just use total_pipeline_latency as the postTimeoffset.
                // This hack will be removed once poseTimeOffset can be calculated more accurately.
                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_input_acquired(tracking.target_timestamp);
                }
            }

            Ok(())
        }
    };

    let statistics_send_loop = {
        let mut socket_sender = stream_socket.request_stream(STATISTICS).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *STATISTICS_SENDER.lock() = Some(data_sender);

            while let Some(stats) = data_receiver.recv().await {
                socket_sender.send(&stats, vec![]).await.ok();
            }

            Ok(())
        }
    };

    let streaming_start_event = ClientCoreEvent::StreamingStarted {
        view_resolution: stream_config.view_resolution,
        fps: stream_config.fps,
        foveated_rendering: settings.video.foveated_rendering.into_option(),
        oculus_foveation_level: settings.video.oculus_foveation_level,
        dynamic_oculus_foveation: settings.video.dynamic_oculus_foveation,
    };

    IS_STREAMING.set(true);

    let video_receive_loop = {
        let mut receiver = stream_socket.subscribe_to_stream::<Duration>(VIDEO).await?;
        let disconnection_critera = settings.connection.disconnection_criteria;
        async move {
            let _decoder_guard = decoder_guard.lock().await;

            // close stream on Drop (manual disconnection or execution canceling)
            struct StreamCloseGuard;

            impl Drop for StreamCloseGuard {
                fn drop(&mut self) {
                    EVENT_QUEUE
                        .lock()
                        .push_back(ClientCoreEvent::StreamingStopped);

                    IS_STREAMING.set(false);

                    #[cfg(target_os = "android")]
                    {
                        *crate::decoder::DECODER_ENQUEUER.lock() = None;
                        *crate::decoder::DECODER_DEQUEUER.lock() = None;
                    }
                }
            }

            let _stream_guard = StreamCloseGuard;

            EVENT_QUEUE.lock().push_back(streaming_start_event);

            let mut receiver_buffer = ReceiverBuffer::new();
            let mut disconnection_timer_begin = None;
            loop {
                receiver.recv_buffer(&mut receiver_buffer).await?;
                let (timestamp, nal) = receiver_buffer.get()?;

                if !IS_RESUMED.value() {
                    break Ok(());
                }

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_video_packet_received(timestamp);
                }

                decoder::push_nal(timestamp, nal);

                if receiver_buffer.had_packet_loss() {
                    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                        sender.send(ClientControlPacket::VideoErrorReport).ok();
                    }
                }

                if let Switch::Enabled(criteria) = &disconnection_critera {
                    if let Some(stats) = &*STATISTICS_MANAGER.lock() {
                        if stats.average_total_pipeline_latency()
                            < Duration::from_millis(criteria.latency_threshold_ms)
                        {
                            disconnection_timer_begin = None;
                        } else {
                            let begin = disconnection_timer_begin.unwrap_or_else(Instant::now);

                            if Instant::now()
                                > begin + Duration::from_secs(criteria.sustain_duration_s)
                            {
                                DISCONNECT_NOTIFIER.notify_one();
                            }

                            disconnection_timer_begin = Some(begin);
                        }
                    }
                }
            }
        }
    };

    let haptics_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<Haptics>(HAPTICS)
            .await?;
        async move {
            loop {
                let haptics = receiver.recv_header_only().await?;

                EVENT_QUEUE.lock().push_back(ClientCoreEvent::Haptics {
                    device_id: haptics.device_id,
                    duration: haptics.duration,
                    frequency: haptics.frequency,
                    amplitude: haptics.amplitude,
                });
            }
        }
    };

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(config) = settings.audio.game_audio {
        let device = AudioDevice::new_output(None, None).map_err(err!())?;

        let game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO).await?;
        Box::pin(audio::play_audio_loop(
            device,
            2,
            stream_config.game_audio_sample_rate,
            config.buffering,
            game_audio_receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if matches!(settings.audio.microphone, Switch::Enabled(_)) {
        let device = AudioDevice::new_input(None).map_err(err!())?;

        let microphone_sender = stream_socket.request_stream(AUDIO).await?;
        Box::pin(audio::record_audio_loop(
            device,
            1,
            false,
            microphone_sender,
        ))
    } else {
        Box::pin(future::pending())
    };

    // Poll for events that need a constant thread (mainly for the JNI env)
    thread::spawn(|| {
        #[cfg(target_os = "android")]
        let vm = platform::vm();
        #[cfg(target_os = "android")]
        let _env = vm.attach_current_thread();

        let mut previous_hmd_battery_status = (0.0, false);
        let mut battery_poll_deadline = Instant::now();

        while IS_STREAMING.value() {
            if battery_poll_deadline < Instant::now() {
                let new_hmd_battery_status = platform::battery_status();

                if new_hmd_battery_status != previous_hmd_battery_status {
                    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                        sender
                            .send(ClientControlPacket::Battery(BatteryPacket {
                                device_id: *HEAD_ID,
                                gauge_value: new_hmd_battery_status.0,
                                is_plugged: new_hmd_battery_status.1,
                            }))
                            .ok();

                        previous_hmd_battery_status = new_hmd_battery_status;
                    }
                }

                battery_poll_deadline += BATTERY_POLL_INTERVAL;
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    let keepalive_sender_loop = {
        let control_sender = Arc::clone(&control_sender);
        async move {
            loop {
                let res = control_sender
                    .lock()
                    .await
                    .send(&ClientControlPacket::KeepAlive)
                    .await;
                if let Err(e) = res {
                    info!("Server disconnected. Cause: {e}");
                    set_hud_message(SERVER_DISCONNECTED_MESSAGE);
                    break Ok(());
                }

                time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let control_send_loop = async move {
        while let Some(packet) = control_channel_receiver.recv().await {
            control_sender.lock().await.send(&packet).await.ok();
        }

        Ok(())
    };

    let control_receive_loop = async move {
        loop {
            match control_receiver.recv().await {
                Ok(ServerControlPacket::InitializeDecoder { config_buffer }) => {
                    decoder::create_decoder(config_buffer);
                }
                Ok(ServerControlPacket::Restarting) => {
                    info!("{SERVER_RESTART_MESSAGE}");
                    set_hud_message(SERVER_RESTART_MESSAGE);
                    break Ok(());
                }
                Ok(ServerControlPacket::ServerPredictionAverage(interval)) => {
                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_server_prediction_average(interval);
                    }
                }
                Ok(_) => (),
                Err(e) => {
                    info!("{SERVER_DISCONNECTED_MESSAGE} Cause: {e}");
                    set_hud_message(SERVER_DISCONNECTED_MESSAGE);
                    break Ok(());
                }
            }
        }
    };

    let receive_loop = async move { stream_socket.receive_loop().await };

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    tokio::select! {
        res = spawn_cancelable(receive_loop) => {
            if let Err(e) = res {
                info!("Server disconnected. Cause: {e}");
            }
            set_hud_message(
                SERVER_DISCONNECTED_MESSAGE
            );

            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => res,
        res = spawn_cancelable(microphone_loop) => res,
        res = spawn_cancelable(tracking_send_loop) => res,
        res = spawn_cancelable(statistics_send_loop) => res,
        res = spawn_cancelable(video_receive_loop) => res,
        res = spawn_cancelable(haptics_receive_loop) => res,
        res = spawn_cancelable(control_send_loop) => res,

        // keep these loops on the current task
        res = keepalive_sender_loop => res,
        res = control_receive_loop => res,

        _ = DISCONNECT_NOTIFIER.notified() => Ok(()),
    }
}
