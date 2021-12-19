mod connection_utils;
mod nal_parser;

use crate::{
    connection::connection_utils::ConnectionError,
    storage,
    streaming_compositor::StreamingCompositor,
    video_decoder::VideoDecoder,
    xr::{XrActionType, XrProfileDesc, XrSession},
    ViewConfig,
};
use alvr_common::{glam::UVec2, prelude::*, ALVR_NAME, ALVR_VERSION};
use alvr_graphics::GraphicsContext;
use alvr_session::SessionDesc;
use alvr_sockets::{
    ClientConfigPacket, ClientControlPacket, ClientHandshakePacket, HeadsetInfoPacket, Input,
    PeerType, ProtoControlSocket, ServerControlPacket, StreamSocketBuilder, VideoFrameHeaderPacket,
    INPUT,
};
use parking_lot::RwLock;
use serde_json as json;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::Mutex, time};

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
    graphics_context: Arc<GraphicsContext>,
    xr_session: Arc<RwLock<XrSession>>,
    video_streaming_components: Arc<RwLock<Option<VideoStreamingComponents>>>,
) -> StrResult {
    let config = storage::load_config()?;
    let hostname = config.hostname;

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
                    info!("Server response: {:?}", message);
                    return Ok(());
                }
                ConnectionError::NetworkUnreachable => {
                    info!("Network unreachable");

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

    let recommended_view_size = {
        // let xr_session = xr_session.read();
        xr_session.read().recommended_view_sizes()[0]
    };

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
            info!("Stream starting");
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {}", e);
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
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
        info!("Server disconnected. Cause: {}", e);
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

    let target_view_size = UVec2::new(
        config_packet.eye_resolution_width,
        config_packet.eye_resolution_height,
    );

    xr_session.write().update_for_stream(
        target_view_size,
        &[
            ("x_press".into(), XrActionType::Binary),
            ("a_press".into(), XrActionType::Binary),
            // todo
        ],
        vec![XrProfileDesc {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            button_bindings: vec![
                ("x_press".into(), "/user/hand/left/x/click".into()),
                ("a_press".into(), "/user/hand/right/a/click".into()),
                // todo
            ],
            tracked: true,
            has_haptics: true,
        }],
        settings.headset.tracking_space,
        openxr::EnvironmentBlendMode::OPAQUE,
    );

    let input_send_loop = {
        let xr_session = Arc::clone(&xr_session);
        let mut socket_sender = stream_socket.request_stream::<Input>(INPUT).await?;
        async move {
            loop {
                let maybe_input = xr_session
                    .read()
                    .get_streaming_input(Duration::from_millis(0)); // todo IMPORTANT: set this using the predicted pipeline length

                if let Ok(input) = maybe_input {
                    // todo

                    // socket_sender
                    //     .send_buffer(socket_sender.new_buffer(&input, 0)?)
                    //     .await
                    //     .ok();
                }
            }

            // Ok(())
        }
    };

    // let compositor = StreamingCompositor::new(Arc::clone(&graphics_context), target_view_size, 1);

    // let video_decoders = vec![VideoDecoder::new(
    //     graphics_context,
    //     settings.video.codec,
    //     target_view_size,
    //     vec![
    //         ("operating-rate".into(), MediacodecDataType::Int32(i32::MAX)),
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

    // *video_streaming_components.write() = VideoStreamingComponents {
    //     compositor,
    //     video_decoders: todo!(),
    //     frame_metadata_receiver: todo!(),
    // };

    Ok(())
}

pub async fn connection_lifecycle_loop(
    graphics_context: Arc<GraphicsContext>,
    xr_session: Arc<RwLock<XrSession>>,
    video_streaming_components: Arc<RwLock<Option<VideoStreamingComponents>>>,
) {
    loop {
        tokio::join!(
            async {
                show_err(
                    connection_pipeline(
                        Arc::clone(&graphics_context),
                        Arc::clone(&xr_session),
                        Arc::clone(&video_streaming_components),
                    )
                    .await,
                );

                // let any running task or socket shutdown
                time::sleep(CLEANUP_PAUSE).await;
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL)
        );
    }
}
