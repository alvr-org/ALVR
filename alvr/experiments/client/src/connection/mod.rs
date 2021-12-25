mod connection_utils;
mod nal_parser;

use crate::{
    connection::{
        connection_utils::ConnectionError,
        nal_parser::{NalParser, NalType},
    },
    storage,
    streaming_compositor::StreamingCompositor,
    video_decoder::VideoDecoder,
    xr::{XrActionType, XrContext, XrProfileDesc, XrSession},
    ViewConfig,
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{glam::UVec2, prelude::*, Haptics, TrackedDeviceType, ALVR_NAME, ALVR_VERSION};
use alvr_graphics::GraphicsContext;
use alvr_session::{AudioDeviceId, CodecType, MediacodecDataType, SessionDesc, TrackingSpace};
use alvr_sockets::{
    spawn_cancelable, ClientConfigPacket, ClientControlPacket, ClientHandshakePacket,
    HeadsetInfoPacket, Input, PeerType, ProtoControlSocket, ServerControlPacket,
    StreamSocketBuilder, VideoFrameHeaderPacket, AUDIO, HAPTICS, INPUT, VIDEO,
};
use futures::{future::BoxFuture, AsyncReadExt};
use parking_lot::RwLock;
use serde_json as json;
use settings_schema::Switch;
use std::{
    future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{futures::Notified, Mutex, Notify},
    time,
};

const CONTROL_CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const PLAYSPACE_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CLEANUP_PAUSE: Duration = Duration::from_millis(500);

pub struct VideoStreamingComponents {
    pub compositor: StreamingCompositor,
    pub video_decoders: Vec<VideoDecoder>,
    pub frame_metadata_receiver: crossbeam_channel::Receiver<VideoFrameHeaderPacket>,
}

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed);
    }
}

async fn connection_pipeline(
    xr_context: Arc<XrContext>,
    graphics_context: Arc<GraphicsContext>,
    xr_session: Arc<RwLock<Option<XrSession>>>,
    video_streaming_components: Arc<RwLock<Option<VideoStreamingComponents>>>,
    standby_status: Arc<AtomicBool>,
    idr_request_notifier: Arc<Notify>,
) -> StrResult {
    let config = storage::load_config()?;
    let hostname = config.hostname;
    error!("hostname: {}", hostname);

    let handshake_packet = ClientHandshakePacket {
        alvr_name: ALVR_NAME.into(),
        version: ALVR_VERSION.clone(),
        device_name: "OpenXR client".into(),
        hostname: hostname.clone(),
        reserved1: "".into(),
        reserved2: "".into(),
    };

    let (mut proto_socket, server_ip) = tokio::select! {
        res = connection_utils::announce_client_loop(handshake_packet) => {
            match res? {
                ConnectionError::ServerMessage(message) => {
                    error!("Server response: {:?}", message);
                    return Ok(());
                }
                ConnectionError::NetworkUnreachable => {
                    error!("Network unreachable");

                    time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;

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

    let recommended_view_size = xr_session.read().as_ref().unwrap().recommended_view_sizes()[0];

    let headset_info = HeadsetInfoPacket {
        recommended_eye_width: recommended_view_size.x,
        recommended_eye_height: recommended_view_size.y,
        available_refresh_rates: vec![90.0], // this can't be known. the server must be reworked.
        preferred_refresh_rate: 90.0,
        reserved: "".into(),
    };

    trace_err!(proto_socket.send(&(headset_info, server_ip)).await)?;
    let config_packet = trace_err!(proto_socket.recv::<ClientConfigPacket>().await)?;

    let (control_sender, mut control_receiver) = proto_socket.split();
    let control_sender = Arc::new(Mutex::new(control_sender));

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => {
            error!("Stream starting");
        }
        Ok(ServerControlPacket::Restarting) => {
            error!("Server restarting");
            return Ok(());
        }
        Err(e) => {
            error!("Server disconnected. Cause: {}", e);
            return Ok(());
        }
        _ => {
            error!("Unexpected packet");
            return Ok(());
        }
    }

    let settings = {
        let mut session_desc = SessionDesc::default();
        session_desc.merge_from_json(&trace_err!(json::from_str(&config_packet.session_desc))?)?;
        session_desc.to_settings()
    };

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
        error!("Server disconnected. Cause: {}", e);
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

    error!("Connected to server");

    let is_connected = Arc::new(AtomicBool::new(true));
    let _stream_guard = StreamCloseGuard {
        is_connected: Arc::clone(&is_connected),
    };

    let target_view_size = UVec2::new(
        config_packet.eye_resolution_width,
        config_packet.eye_resolution_height,
    );

    {
        let xr_session_ref = &mut *xr_session.write();

        // The Oculus Quest supports creating only one session at a time. Makes sure the old session
        // is destroyed before recreating it.
        *xr_session_ref = None;

        let maybe_new_session = XrSession::new(
            Arc::clone(&xr_context),
            Arc::clone(&graphics_context),
            target_view_size,
            &[
                ("x_press".into(), XrActionType::Binary),
                ("a_press".into(), XrActionType::Binary),
                // todo
            ],
            vec![XrProfileDesc {
                profile: "/interaction_profiles/oculus/touch_controller".into(),
                button_bindings: vec![
                    ("x_press".into(), "/user/hand/left/input/x/click".into()),
                    ("a_press".into(), "/user/hand/right/input/a/click".into()),
                    // todo
                ],
                tracked: true,
                has_haptics: true,
            }],
            settings.headset.tracking_space,
            openxr::EnvironmentBlendMode::OPAQUE,
        );

        *xr_session_ref = match maybe_new_session {
            Ok(session) => Some(session),
            Err(e) => {
                error!("Error recreating session for stream: {}", e);

                // recreate a session for presenting the lobby room
                Some(
                    XrSession::new(
                        Arc::clone(&xr_context),
                        Arc::clone(&graphics_context),
                        UVec2::new(1, 1),
                        &[],
                        vec![],
                        TrackingSpace::Local,
                        openxr::EnvironmentBlendMode::OPAQUE,
                    )
                    .unwrap(),
                )
            }
        };
    }

    // let input_send_loop = {
    //     let xr_session = Arc::clone(&xr_session);
    //     let mut socket_sender = stream_socket.request_stream::<Input>(INPUT).await?;
    //     async move {
    //         loop {
    //             let maybe_input = xr_session
    //                 .read()
    //                 .get_streaming_input(Duration::from_millis(0)); // todo IMPORTANT: set this using the predicted pipeline length

    //             if let Ok(input) = maybe_input {
    //                 // todo

    //                 // socket_sender
    //                 //     .send_buffer(socket_sender.new_buffer(&input, 0)?)
    //                 //     .await
    //                 //     .ok();
    //             }
    //         }

    //         // Ok(())
    //     }
    // };

    let video_receive_loop = {
        let video_streaming_components = Arc::clone(&video_streaming_components);
        let standby_status = Arc::clone(&standby_status);
        let mut receiver = stream_socket
            .subscribe_to_stream::<VideoFrameHeaderPacket>(VIDEO)
            .await?;
        // let frame_metadata_sender = None;
        let nal_parser = NalParser::new(settings.video.codec);
        async move {
            loop {
                let packet = receiver.recv().await?;

                let nals = nal_parser.process_packet(packet.buffer.to_vec());

                for (nal_type, buffer) in nals {
                    match nal_type {
                        NalType::Config => {
                            if video_streaming_components.read().is_none() {
                                let compositor = StreamingCompositor::new(
                                    Arc::clone(&graphics_context),
                                    target_view_size,
                                    1,
                                );

                                // let video_decoders = vec![VideoDecoder::new(
                                //     Arc::clone(&graphics_context),
                                //     settings.video.codec,
                                //     target_view_size,
                                //     buffer,
                                //     &[
                                //         (
                                //             "operating-rate".into(),
                                //             MediacodecDataType::Int32(i32::MAX),
                                //         ),
                                //         ("priority".into(), MediacodecDataType::Int32(0)),
                                //         // low-latency: only applicable on API level 30. Quest 1 and 2 might not be
                                //         // cabable, since they are on level 29.
                                //         ("low-latency".into(), MediacodecDataType::Int32(1)),
                                //         (
                                //             "vendor.qti-ext-dec-low-latency.enable".into(),
                                //             MediacodecDataType::Int32(1),
                                //         ),
                                //     ],
                                // )?];

                                // let (metadata_sender, metadata_receiver) =
                                //     crossbeam_channel::unbounded();

                                // frame_metadata_sender = Some(metadata_sender);

                                // *video_streaming_components.write() =
                                //     Some(VideoStreamingComponents {
                                //         compositor,
                                //         video_decoders,
                                //         frame_metadata_receiver: metadata_receiver,
                                //     });
                            }
                        }
                        NalType::Frame => {
                            if !standby_status.load(Ordering::Relaxed) {
                                if let Some(streaming_components) =
                                    &*video_streaming_components.read()
                                {
                                    //     let timestamp =
                                    //         Duration::from_nanos(packet.header.packet_counter as _); // fixme: this is nonsensical

                                    //     frame_metadata_sender.unwrap().send(packet.header);
                                    //     streaming_components.video_decoders[0].push_frame_nals(
                                    //         timestamp,
                                    //         &buffer,
                                    //         Duration::SECOND,
                                    //     )?
                                } else {
                                    error!("Frame discarded because decoder is not initialized");
                                }
                            } else {
                                error!("Frame discarded because in standby");
                            }
                        }
                    }
                }
            }
        }
    };

    let haptics_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<Haptics<TrackedDeviceType>>(HAPTICS)
            .await?;

        async move {
            loop {
                let packet = receiver.recv().await?;

                // todo
            }
        }
    };

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO).await?;

        Box::pin(alvr_audio::play_audio_loop(
            AudioDevice::new(AudioDeviceId::Default, AudioDeviceType::Output)?,
            2,
            config_packet.game_audio_sample_rate,
            desc.config,
            game_audio_receiver,
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
                    error!("Server disconnected. Cause: {}", e);
                    break Ok(());
                }

                time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let control_loop = {
        async move {
            loop {
                tokio::select! {
                    _ = idr_request_notifier.notified() => {
                        control_sender.lock().await.send(&ClientControlPacket::RequestIdr).await?;
                    }
                    control_packet = control_receiver.recv() =>
                        match control_packet {
                            Ok(ServerControlPacket::Restarting) => {
                                error!("Server restarting");
                                break Ok(());
                            }
                            Ok(_) => (),
                            Err(e) => {
                                error!("Server disconnected. Cause: {}", e);
                                break Ok(());
                            }
                        }
                }
            }
        }
    };

    let receive_loop = async move { stream_socket.receive_loop().await };

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    tokio::select! {
        res = spawn_cancelable(receive_loop) => {
            if let Err(e) = res {
                error!("Server disconnected. Cause: {}", e);
            }

            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => res,
        // res = spawn_cancelable(microphone_loop) => res,
        // res = spawn_cancelable(tracking_loop) => res,
        // res = spawn_cancelable(playspace_sync_loop) => res,
        // res = spawn_cancelable(input_send_loop) => res,
        // res = spawn_cancelable(time_sync_send_loop) => res,
        // res = spawn_cancelable(video_error_report_send_loop) => res,
        res = spawn_cancelable(video_receive_loop) => res,
        res = spawn_cancelable(haptics_receive_loop) => res,
        // res = legacy_stream_socket_loop => trace_err!(res)?,

        // keep these loops on the current task
        res = keepalive_sender_loop => res,
        res = control_loop => res,
        // res = debug_loop => res,
    }
}

pub async fn connection_lifecycle_loop(
    xr_context: Arc<XrContext>,
    graphics_context: Arc<GraphicsContext>,
    xr_session: Arc<RwLock<Option<XrSession>>>,
    video_streaming_components: Arc<RwLock<Option<VideoStreamingComponents>>>,
    standby_status: Arc<AtomicBool>,
    idr_request_notifier: Arc<Notify>,
) {
    loop {
        tokio::join!(
            async {
                show_err(
                    connection_pipeline(
                        Arc::clone(&xr_context),
                        Arc::clone(&graphics_context),
                        Arc::clone(&xr_session),
                        Arc::clone(&video_streaming_components),
                        Arc::clone(&standby_status),
                        Arc::clone(&idr_request_notifier),
                    )
                    .await,
                );

                // stop streming receiver and return to lobby
                video_streaming_components.write().take();

                // let any running task or socket shutdown
                time::sleep(CLEANUP_PAUSE).await;
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL)
        );
    }
}
