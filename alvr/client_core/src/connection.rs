#![allow(clippy::if_same_then_else)]

use crate::{
    connection_utils::{self, ConnectionError},
    platform,
    statistics::StatisticsManager,
    storage::Config,
    AlvrEvent, VideoFrame, CONTROL_CHANNEL_SENDER, DECODER_DEQUEUER, DECODER_ENQUEUER,
    DECODER_INIT_CONFIG, DISCONNECT_NOTIFIER, EVENT_QUEUE, IS_RESUMED, IS_STREAMING,
    STATISTICS_MANAGER, STATISTICS_SENDER, TRACKING_SENDER, USE_OPENGL,
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{prelude::*, ALVR_NAME, ALVR_VERSION};
use alvr_session::{
    AudioDeviceId, CodecType, MediacodecDataType, OculusFovetionLevel, SessionDesc,
};
use alvr_sockets::{
    spawn_cancelable, ClientConfigPacket, ClientConnectionResult, ClientControlPacket,
    ClientHandshakePacket, Haptics, HeadsetInfoPacket, PeerType, ProtoControlSocket,
    ServerControlPacket, ServerHandshakePacket, StreamSocketBuilder, VideoFrameHeaderPacket, AUDIO,
    HAPTICS, STATISTICS, TRACKING, VIDEO,
};
use futures::future::BoxFuture;
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use serde_json as json;
use settings_schema::Switch;
use std::{future, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc as tmpsc, Mutex},
    time,
};

#[cfg(target_os = "android")]
use crate::audio;
#[cfg(not(target_os = "android"))]
use alvr_audio as audio;

const INITIAL_MESSAGE: &str = "Searching for server...\n(open ALVR on your PC)";
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
const CLIENT_UNTRUSTED_MESSAGE: &str = "On the PC, click \"Trust\"\nnext to the client entry";
const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
    "Server and client have\n",
    "incompatible types.\n",
    "Please update either the app\n",
    "on the PC or on the headset"
);
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";

const CONTROL_CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CLEANUP_PAUSE: Duration = Duration::from_millis(500);

const LOADING_TEXTURE_WIDTH: usize = 1280;
const LOADING_TEXTURE_HEIGHT: usize = 720;
const FONT_SIZE: f32 = 50_f32;

fn set_loading_message(message: &str) {
    let hostname = Config::load().hostname;

    let message = format!(
        "ALVR v{}\nhostname: {hostname}\n \n{message}",
        *ALVR_VERSION,
    );

    let ubuntu_font =
        FontRef::try_from_slice(include_bytes!("../resources/Ubuntu-Medium.ttf")).unwrap();

    let section_glyphs = Layout::default()
        .h_align(HorizontalAlign::Center)
        .v_align(VerticalAlign::Center)
        .calculate_glyphs(
            &[&ubuntu_font],
            &SectionGeometry {
                screen_position: (
                    LOADING_TEXTURE_WIDTH as f32 / 2_f32,
                    LOADING_TEXTURE_HEIGHT as f32 / 2_f32,
                ),
                ..Default::default()
            },
            &[SectionText {
                text: &message,
                scale: FONT_SIZE.into(),
                font_id: FontId(0),
            }],
        );

    let scaled_font = ubuntu_font.as_scaled(FONT_SIZE);

    let mut buffer = vec![0_u8; LOADING_TEXTURE_WIDTH * LOADING_TEXTURE_HEIGHT * 4];

    for section_glyph in section_glyphs {
        if let Some(outlined) = scaled_font.outline_glyph(section_glyph.glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, alpha| {
                let x = x as usize + bounds.min.x as usize;
                let y = y as usize + bounds.min.y as usize;
                buffer[(y * LOADING_TEXTURE_WIDTH + x) * 4 + 3] = (alpha * 255.0) as u8;
            });
        }
    }

    if USE_OPENGL.value() {
        unsafe { crate::updateLobbyHudTexture(buffer.as_ptr()) };
    }
}

async fn connection_pipeline(
    headset_info: HeadsetInfoPacket,
    decoder_guard: Arc<Mutex<()>>,
) -> StrResult {
    let device_name = platform::device_name();
    let hostname = Config::load().hostname;

    let handshake_packet = ClientHandshakePacket {
        alvr_name: ALVR_NAME.into(),
        version: ALVR_VERSION.clone(),
        device_name,
        hostname,
        reserved1: "".into(),
        reserved2: "".into(),
    };

    let (mut proto_socket, server_ip) = tokio::select! {
        res = connection_utils::announce_client_loop(handshake_packet) => {
            match res? {
                ConnectionError::ServerMessage(message) => {
                    info!("Server response: {message:?}");
                    let message_str = match message {
                        ServerHandshakePacket::ClientUntrusted => CLIENT_UNTRUSTED_MESSAGE,
                        ServerHandshakePacket::IncompatibleVersions =>
                            INCOMPATIBLE_VERSIONS_MESSAGE,
                    };
                    set_loading_message(message_str);
                    return Ok(());
                }
                ConnectionError::NetworkUnreachable => {
                    info!("Network unreachable");
                    set_loading_message(
                        NETWORK_UNREACHABLE_MESSAGE,
                    );

                    time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;

                    set_loading_message(
                        INITIAL_MESSAGE,
                    );

                    return Ok(());
                }
            }
        },
        pair = async {
            loop {
                if let Ok(pair) = ProtoControlSocket::connect_to(PeerType::Server).await {
                    break pair;
                }

                time::sleep(CONTROL_CONNECT_RETRY_PAUSE).await;
            }
        } => pair
    };

    if !IS_RESUMED.value() {
        info!("Not streaming because not resumed");
        proto_socket
            .send(&ClientConnectionResult::ClientStandby)
            .await
            .map_err(err!())?;
        return Ok(());
    }

    proto_socket
        .send(&ClientConnectionResult::ServerAccepted {
            headset_info,
            server_ip,
        })
        .await
        .map_err(err!())?;
    let config_packet = proto_socket
        .recv::<ClientConfigPacket>()
        .await
        .map_err(err!())?;

    let (control_sender, mut control_receiver) = proto_socket.split();
    let control_sender = Arc::new(Mutex::new(control_sender));

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => {
            info!("Stream starting");
            set_loading_message(STREAM_STARTING_MESSAGE);
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_loading_message(SERVER_RESTART_MESSAGE);
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {e}");
            set_loading_message(SERVER_DISCONNECTED_MESSAGE);
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_loading_message("Unexpected packet");
            return Ok(());
        }
    }

    let settings = {
        let mut session_desc = SessionDesc::default();
        session_desc
            .merge_from_json(&json::from_str(&config_packet.session_desc).map_err(err!())?)?;
        session_desc.to_settings()
    };

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size as _,
    ));

    let stream_socket_builder = StreamSocketBuilder::listen_for_server(
        settings.connection.stream_port,
        settings.connection.stream_protocol,
    )
    .await?;

    if let Err(e) = control_sender
        .lock()
        .await
        .send(&ClientControlPacket::StreamReady)
        .await
    {
        info!("Server disconnected. Cause: {e}");
        set_loading_message(SERVER_DISCONNECTED_MESSAGE);
        return Ok(());
    }

    let stream_socket = tokio::select! {
        res = stream_socket_builder.accept_from_server(
            server_ip,
            settings.connection.stream_port,
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

        config.options = vec![
            ("operating-rate".into(), MediacodecDataType::Int32(i32::MAX)),
            ("priority".into(), MediacodecDataType::Int32(0)),
            // low-latency: only applicable on API level 30. Quest 1 and 2 might not be
            // cabable, since they are on level 29.
            ("low-latency".into(), MediacodecDataType::Int32(1)),
            (
                "vendor.qti-ext-dec-low-latency.enable".into(),
                MediacodecDataType::Int32(1),
            ),
        ];
    }

    unsafe {
        crate::setStreamConfig(crate::StreamConfigInput {
            viewWidth: config_packet.view_resolution_width,
            viewHeight: config_packet.view_resolution_height,
            enableFoveation: matches!(settings.video.foveated_rendering, Switch::Enabled(_)),
            foveationCenterSizeX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_size_x
            } else {
                3_f32 / 5_f32
            },
            foveationCenterSizeY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_size_y
            } else {
                2_f32 / 5_f32
            },
            foveationCenterShiftX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_shift_x
            } else {
                2_f32 / 5_f32
            },
            foveationCenterShiftY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_shift_y
            } else {
                1_f32 / 10_f32
            },
            foveationEdgeRatioX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.edge_ratio_x
            } else {
                2_f32
            },
            foveationEdgeRatioY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.edge_ratio_y
            } else {
                2_f32
            },
        });
    }

    // setup stream loops

    // let (debug_sender, mut debug_receiver) = tmpsc::unbounded_channel();
    // let debug_loop = {
    //     let control_sender = Arc::clone(&control_sender);
    //     async move {
    //         while let Some(data) = debug_receiver.recv().await {
    //             control_sender
    //                 .lock()
    //                 .await
    //                 .send(&ClientControlPacket::Reserved(data))
    //                 .await
    //                 .ok();
    //         }

    //         Ok(())
    //     }
    // };

    let tracking_send_loop = {
        let mut socket_sender = stream_socket.request_stream(TRACKING).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *TRACKING_SENDER.lock() = Some(data_sender);
            while let Some(tracking) = data_receiver.recv().await {
                socket_sender
                    .send_buffer(socket_sender.new_buffer(&tracking, 0)?)
                    .await
                    .ok();

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
                socket_sender
                    .send_buffer(socket_sender.new_buffer(&stats, 0)?)
                    .await
                    .ok();
            }

            Ok(())
        }
    };

    let streaming_start_event = AlvrEvent::StreamingStarted {
        view_width: config_packet.view_resolution_width as _,
        view_height: config_packet.view_resolution_height as _,
        fps: config_packet.fps,
        oculus_foveation_level: if let Switch::Enabled(foveation_vars) =
            &settings.video.foveated_rendering
        {
            foveation_vars.oculus_foveation_level
        } else {
            OculusFovetionLevel::None
        } as i32,
        dynamic_oculus_foveation: if let Switch::Enabled(foveation_vars) =
            &settings.video.foveated_rendering
        {
            foveation_vars.dynamic_oculus_foveation
        } else {
            false
        },
        extra_latency: settings.headset.extra_latency_mode,
        controller_prediction_multiplier: settings
            .headset
            .controllers
            .into_option()
            .map(|c| c.prediction_multiplier)
            .unwrap_or_default(),
    };

    let video_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<VideoFrameHeaderPacket>(VIDEO)
            .await?;
        let codec = settings.video.codec;
        let enable_fec = settings.connection.enable_fec;
        async move {
            let _decoder_guard = decoder_guard.lock().await;

            // close stream on Drop (manual disconnection or execution canceling)
            struct StreamCloseGuard;

            impl Drop for StreamCloseGuard {
                fn drop(&mut self) {
                    EVENT_QUEUE.lock().push_back(AlvrEvent::StreamingStopped);

                    IS_STREAMING.set(false);

                    *DECODER_ENQUEUER.lock() = None;
                    *DECODER_DEQUEUER.lock() = None;
                }
            }

            let _stream_guard = StreamCloseGuard;

            unsafe {
                crate::initializeNalParser(matches!(codec, CodecType::HEVC) as _, enable_fec)
            };

            IS_STREAMING.set(true);

            EVENT_QUEUE.lock().push_back(streaming_start_event);

            loop {
                let packet = receiver.recv().await?;

                if !IS_RESUMED.value() {
                    break Ok(());
                }

                let header = VideoFrame {
                    packetCounter: packet.header.packet_counter,
                    trackingFrameIndex: packet.header.tracking_frame_index,
                    videoFrameIndex: packet.header.video_frame_index,
                    sentTime: packet.header.sent_time,
                    frameByteSize: packet.header.frame_byte_size,
                    fecIndex: packet.header.fec_index,
                    fecPercentage: packet.header.fec_percentage,
                };

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_video_packet_received(Duration::from_nanos(
                        packet.header.tracking_frame_index,
                    ));
                }

                let mut fec_failure = false;
                unsafe {
                    crate::processNalPacket(
                        header,
                        packet.buffer.as_ptr(),
                        packet.buffer.len() as _,
                        &mut fec_failure,
                    )
                };
                if fec_failure {
                    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                        sender.send(ClientControlPacket::VideoErrorReport).ok();
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
                let packet = receiver.recv().await?.header;

                EVENT_QUEUE.lock().push_back(AlvrEvent::Haptics {
                    device_id: packet.path,
                    duration_s: packet.duration.as_secs_f32(),
                    frequency: packet.frequency,
                    amplitude: packet.amplitude,
                });
            }
        }
    };

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let device = AudioDevice::new(None, &AudioDeviceId::Default, AudioDeviceType::Output)
            .map_err(err!())?;

        let game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO).await?;
        Box::pin(audio::play_audio_loop(
            device,
            2,
            config_packet.game_audio_sample_rate,
            desc.buffering_config,
            game_audio_receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if matches!(settings.audio.microphone, Switch::Enabled(_)) {
        let device = AudioDevice::new(None, &AudioDeviceId::Default, AudioDeviceType::Input)
            .map_err(err!())?;

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
                    set_loading_message(SERVER_DISCONNECTED_MESSAGE);
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
                Ok(ServerControlPacket::Restarting) => {
                    info!("{SERVER_RESTART_MESSAGE}");
                    set_loading_message(SERVER_RESTART_MESSAGE);
                    break Ok(());
                }
                Ok(_) => (),
                Err(e) => {
                    info!("{SERVER_DISCONNECTED_MESSAGE} Cause: {e}");
                    set_loading_message(SERVER_DISCONNECTED_MESSAGE);
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
            set_loading_message(
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
        // res = debug_loop => res,

        _ = DISCONNECT_NOTIFIER.notified() => Ok(()),
    }
}

pub async fn connection_lifecycle_loop(headset_info: HeadsetInfoPacket) {
    set_loading_message(INITIAL_MESSAGE);

    let decoder_guard = Arc::new(Mutex::new(()));

    loop {
        tokio::join!(
            async {
                let maybe_error =
                    connection_pipeline(headset_info.clone(), Arc::clone(&decoder_guard)).await;

                if let Err(e) = maybe_error {
                    let message = format!("Connection error:\n{e}\nCheck the PC for more details");
                    error!("{message}");
                    set_loading_message(&message);
                }

                // let any running task or socket shutdown
                time::sleep(CLEANUP_PAUSE).await;
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}
