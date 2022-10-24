#![allow(clippy::if_same_then_else)]

use crate::{
    decoder::DECODER_INIT_CONFIG, platform, sockets::AnnouncerSocket,
    statistics::StatisticsManager, storage::Config, AlvrEvent, VideoFrame, CONTROL_CHANNEL_SENDER,
    DISCONNECT_NOTIFIER, EVENT_QUEUE, IS_ALIVE, IS_RESUMED, IS_STREAMING, STATISTICS_MANAGER,
    STATISTICS_SENDER, TRACKING_SENDER,
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{glam::UVec2, prelude::*, ALVR_VERSION};
use alvr_session::{AudioDeviceId, CodecType, OculusFovetionLevel, SessionDesc};
use alvr_sockets::{
    spawn_cancelable, ClientConnectionResult, ClientControlPacket, Haptics, PeerType,
    ProtoControlSocket, ServerControlPacket, StreamConfigPacket, StreamSocketBuilder,
    VideoFrameHeaderPacket, VideoStreamingCapabilities, AUDIO, HAPTICS, STATISTICS, TRACKING,
    VIDEO,
};
use futures::future::BoxFuture;
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use serde_json as json;
use settings_schema::Switch;
use std::{future, net::IpAddr, sync::Arc, thread, time::Duration};
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
    "Searching for server...\n",
    "Open ALVR on your PC then click \"Trust\"\n",
    "next to the client entry",
);
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
    "Server and client have\n",
    "incompatible types.\n",
    "Please update either the app\n",
    "on the PC or on the headset",
);
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";

const DISCOVERY_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CONNECTION_ERROR_PAUSE: Duration = Duration::from_millis(500);

const HUD_TEXTURE_WIDTH: usize = 1280;
const HUD_TEXTURE_HEIGHT: usize = 720;
const FONT_SIZE: f32 = 50_f32;

fn set_hud_message(message: &str) {
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
                    HUD_TEXTURE_WIDTH as f32 / 2_f32,
                    HUD_TEXTURE_HEIGHT as f32 / 2_f32,
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

    let mut buffer = vec![0_u8; HUD_TEXTURE_WIDTH * HUD_TEXTURE_HEIGHT * 4];

    for section_glyph in section_glyphs {
        if let Some(outlined) = scaled_font.outline_glyph(section_glyph.glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, alpha| {
                let x = x as usize + bounds.min.x as usize;
                let y = y as usize + bounds.min.y as usize;
                buffer[(y * HUD_TEXTURE_WIDTH + x) * 4 + 3] = (alpha * 255.0) as u8;
            });
        }
    }

    #[cfg(target_os = "android")]
    unsafe {
        crate::updateLobbyHudTexture(buffer.as_ptr())
    };
}

pub fn connection_lifecycle_loop(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
) -> IntResult {
    set_hud_message(INITIAL_MESSAGE);

    let decoder_guard = Arc::new(Mutex::new(()));

    loop {
        check_interrupt!(IS_ALIVE.value());

        match connection_pipeline(
            recommended_view_resolution,
            supported_refresh_rates.clone(),
            Arc::clone(&decoder_guard),
        ) {
            Ok(()) => continue,
            Err(InterruptibleError::Interrupted) => return Ok(()),
            Err(InterruptibleError::Other(e)) => {
                let message = format!("Connection error:\n{e}\nCheck the PC for more details");
                error!("{message}");
                set_hud_message(&message);

                // avoid spamming error messages
                thread::sleep(CONNECTION_ERROR_PAUSE);
            }
        }
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

    if !IS_RESUMED.value() {
        info!("Not streaming because not resumed");
        return runtime
            .block_on(proto_control_socket.send(&ClientConnectionResult::ClientStandby))
            .map_err(to_int_e!());
    }

    let microphone_sample_rate =
        AudioDevice::new(None, &AudioDeviceId::Default, AudioDeviceType::Input)
            .unwrap()
            .input_sample_rate()
            .unwrap();

    runtime
        .block_on(
            proto_control_socket.send(&ClientConnectionResult::ConnectionAccepted {
                display_name: platform::device_name(),
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

    #[cfg(target_os = "android")]
    unsafe {
        crate::setStreamConfig(crate::StreamConfigInput {
            viewWidth: stream_config.view_resolution.x,
            viewHeight: stream_config.view_resolution.y,
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
        view_width: stream_config.view_resolution.x,
        view_height: stream_config.view_resolution.y,
        fps: stream_config.fps,
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
            .clientside_controller_prediction_multiplier,
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

                    #[cfg(target_os = "android")]
                    {
                        *crate::decoder::DECODER_ENQUEUER.lock() = None;
                        *crate::decoder::DECODER_DEQUEUER.lock() = None;
                    }
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
            stream_config.game_audio_sample_rate,
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
                Ok(ServerControlPacket::Restarting) => {
                    info!("{SERVER_RESTART_MESSAGE}");
                    set_hud_message(SERVER_RESTART_MESSAGE);
                    break Ok(());
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
