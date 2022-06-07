#![allow(clippy::if_same_then_else)]

use crate::{
    connection_utils::{self, ConnectionError},
    decoder::{DECODER_REF, IDR_PARSED},
    platform,
    statistics::StatisticsManager,
    VideoFrame, CONTROL_CHANNEL_SENDER, INPUT_SENDER, STATISTICS_MANAGER, STATISTICS_SENDER,
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{glam::Vec2, prelude::*, ALVR_NAME, ALVR_VERSION};
use alvr_session::{AudioDeviceId, CodecType, SessionDesc};
use alvr_sockets::{
    spawn_cancelable, ClientConfigPacket, ClientControlPacket, ClientHandshakePacket, Haptics,
    HeadsetInfoPacket, PeerType, ProtoControlSocket, ServerControlPacket, ServerHandshakePacket,
    StreamSocketBuilder, VideoFrameHeaderPacket, AUDIO, HAPTICS, INPUT, STATISTICS, VIDEO,
};
use futures::future::BoxFuture;
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use serde_json as json;
use settings_schema::Switch;
use std::{
    future, mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc, Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc as tmpsc, Mutex},
    task,
    time::{self, Instant},
};

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

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed);
    }
}

fn set_loading_message(message: &str) {
    let hostname = platform::load_config().hostname;

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

    unsafe { crate::updateLoadingTexuture(buffer.as_ptr()) };
}

fn on_server_connected(fps: f32, codec: CodecType, realtime_decoder: bool) {
    let vm = platform::vm();
    let env = vm.attach_current_thread().unwrap();

    env.call_method(
        platform::context(),
        "onServerConnected",
        "(FIZ)V",
        &[
            fps.into(),
            (matches!(codec, CodecType::HEVC) as i32).into(),
            realtime_decoder.into(),
        ],
    )
    .unwrap();
}

async fn connection_pipeline(headset_info: &HeadsetInfoPacket) -> StrResult {
    let device_name = platform::device_name();
    let hostname = platform::load_config().hostname;

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

    proto_socket
        .send(&(headset_info, server_ip))
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

    let is_connected = Arc::new(AtomicBool::new(true));
    let _stream_guard = StreamCloseGuard {
        is_connected: Arc::clone(&is_connected),
    };

    {
        let mut config = platform::load_config();
        config.dark_mode = settings.extra.client_dark_mode;
        platform::store_config(&config);
    }

    // create this before initializing the stream on cpp side
    let (control_channel_sender, mut control_channel_receiver) = tmpsc::unbounded_channel();
    *CONTROL_CHANNEL_SENDER.lock() = Some(control_channel_sender);

    unsafe {
        crate::setStreamConfig(crate::StreamConfigInput {
            eyeWidth: config_packet.eye_resolution_width,
            eyeHeight: config_packet.eye_resolution_height,
            refreshRate: config_packet.fps,
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
            oculusFoveationLevel: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.oculus_foveation_level as i32
            } else {
                0
            },
            dynamicOculusFoveation: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.dynamic_oculus_foveation
            } else {
                false
            },
            extraLatencyMode: settings.headset.extra_latency_mode,
            clientsidePrediction: settings
                .headset
                .controllers
                .into_option()
                .map(|c| c.clientside_prediction)
                .unwrap_or(false),
        });
    }

    on_server_connected(
        config_packet.fps,
        settings.video.codec,
        settings.video.client_request_realtime_decoder,
    );

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

    let input_send_loop = {
        let mut socket_sender = stream_socket.request_stream(INPUT).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *INPUT_SENDER.lock() = Some(data_sender);
            while let Some(input) = data_receiver.recv().await {
                socket_sender
                    .send_buffer(socket_sender.new_buffer(&input, 0)?)
                    .await
                    .ok();

                // Note: this is not the best place to report the acquired input. Instead it should
                // be done as soon as possible (or even just before polling the input). Instead this
                // is reported late to partially compensate for lack of network latency measurement,
                // so the server can just use total_pipeline_latency as the postTimeoffset.
                // This hack will be removed once poseTimeOffset can be calculated more accurately.
                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_input_acquired(input.target_timestamp);
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

    let (legacy_receive_data_sender, legacy_receive_data_receiver) = smpsc::channel();
    let legacy_receive_data_sender = Arc::new(Mutex::new(legacy_receive_data_sender));

    let video_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<VideoFrameHeaderPacket>(VIDEO)
            .await?;
        let legacy_receive_data_sender = legacy_receive_data_sender.clone();
        async move {
            loop {
                let packet = receiver.recv().await?;

                let mut buffer = vec![0_u8; mem::size_of::<VideoFrame>() + packet.buffer.len()];
                let header = VideoFrame {
                    type_: 9, // ALVR_PACKET_TYPE_VIDEO_FRAME
                    packetCounter: packet.header.packet_counter,
                    trackingFrameIndex: packet.header.tracking_frame_index,
                    videoFrameIndex: packet.header.video_frame_index,
                    sentTime: packet.header.sent_time,
                    frameByteSize: packet.header.frame_byte_size,
                    fecIndex: packet.header.fec_index,
                    fecPercentage: packet.header.fec_percentage,
                };

                buffer[..mem::size_of::<VideoFrame>()].copy_from_slice(unsafe {
                    &mem::transmute::<_, [u8; mem::size_of::<VideoFrame>()]>(header)
                });
                buffer[mem::size_of::<VideoFrame>()..].copy_from_slice(&packet.buffer);

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_video_packet_received(Duration::from_nanos(
                        packet.header.tracking_frame_index,
                    ));
                }

                legacy_receive_data_sender.lock().await.send(buffer).ok();
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

                unsafe {
                    crate::hapticsFeedbackNative(
                        packet.path,
                        packet.duration.as_secs_f32(),
                        packet.frequency,
                        packet.amplitude,
                    )
                };
            }
        }
    };

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function exits or gets canceled, this loop will run to finish.
    let legacy_stream_socket_loop = task::spawn_blocking({
        let codec = settings.video.codec;
        let enable_fec = settings.connection.enable_fec;
        move || -> StrResult {
            unsafe {
                // Note: legacyReceive() requires the java context to be attached to the current thread
                // todo: investigate why
                let vm = platform::vm();
                let env = vm.attach_current_thread().unwrap();

                crate::initializeSocket(matches!(codec, CodecType::HEVC) as _, enable_fec);

                let mut idr_request_deadline = None;

                while let Ok(mut data) = legacy_receive_data_receiver.recv() {
                    // Send again IDR packet every 2s in case it is missed
                    // (due to dropped burst of packets at the start of the stream or otherwise).
                    if !IDR_PARSED.load(Ordering::Relaxed) {
                        if let Some(deadline) = idr_request_deadline {
                            if deadline < Instant::now() {
                                if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                                    sender.send(ClientControlPacket::RequestIdr).ok();
                                }
                                idr_request_deadline = None;
                            }
                        } else {
                            idr_request_deadline = Some(Instant::now() + Duration::from_secs(2));
                        }
                    }

                    crate::legacyReceive(data.as_mut_ptr(), data.len() as _);
                }

                crate::closeSocket();

                if let Some(decoder) = &*DECODER_REF.lock() {
                    env.call_method(decoder.as_obj(), "onDisconnect", "()V", &[])
                        .unwrap();
                }
            }

            Ok(())
        }
    });

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let device = AudioDevice::new(None, AudioDeviceId::Default, AudioDeviceType::Output)
            .map_err(err!())?;

        let game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO).await?;
        Box::pin(alvr_audio::play_audio_loop(
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
        let device = AudioDevice::new(None, AudioDeviceId::Default, AudioDeviceType::Input)
            .map_err(err!())?;

        let microphone_sender = stream_socket.request_stream(AUDIO).await?;
        Box::pin(alvr_audio::record_audio_loop(
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
        res = spawn_cancelable(input_send_loop) => res,
        res = spawn_cancelable(statistics_send_loop) => res,
        res = spawn_cancelable(video_receive_loop) => res,
        res = spawn_cancelable(haptics_receive_loop) => res,
        res = spawn_cancelable(control_send_loop) => res,
        res = legacy_stream_socket_loop => res.map_err(err!())?,

        // keep these loops on the current task
        res = keepalive_sender_loop => res,
        res = control_receive_loop => res,
        // res = debug_loop => res,
    }
}

pub async fn connection_lifecycle_loop(headset_info: HeadsetInfoPacket) {
    set_loading_message(INITIAL_MESSAGE);

    loop {
        tokio::join!(
            async {
                let maybe_error = connection_pipeline(&headset_info).await;

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
