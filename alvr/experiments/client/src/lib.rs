mod connection;
mod openxr;
mod scene;
mod streaming_compositor;
mod video_decoder;

use crate::openxr::{OpenxrContext, OpenxrPresentationGuard, OpenxrSession};
use alvr_common::{
    glam::{Quat, Vec3},
    prelude::*,
    Fov,
};
use alvr_graphics::GraphicsContext;
use connection::FrameMetadataPacket;
use parking_lot::{Mutex, RwLock};
use scene::Scene;
use std::{sync::Arc, time::Duration};
use streaming_compositor::StreamingCompositor;
use video_decoder::VideoDecoder;
use wgpu::Texture;

// Timeout stream after this portion of frame interval. must be less than 1, so Phase Sync can
// compensate for it.
const FRAME_TIMEOUT_MULTIPLIER: f32 = 0.9;

#[derive(Clone)]
pub struct ViewConfig {
    orientation: Quat,
    position: Vec3,
    fov: Fov,
}

struct VideoStreamingComponents {
    compositor: StreamingCompositor,
    video_decoders: Vec<VideoDecoder>,
    frame_metadata_receiver: crossbeam_channel::Receiver<FrameMetadataPacket>,
}

#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {
    show_err(run());

    #[cfg(target_os = "android")]
    ndk_glue::native_activity().finish();
}

fn run() -> StrResult {
    let xr_context = Arc::new(OpenxrContext::new());

    let graphics_context = Arc::new(openxr::create_graphics_context(&xr_context)?);

    loop {
        let res = show_err(session_pipeline(
            Arc::clone(&xr_context),
            Arc::clone(&graphics_context),
        ));

        if res.is_some() {
            break Ok(());
        }
    }
}

fn session_pipeline(
    xr_context: Arc<OpenxrContext>,
    graphics_context: Arc<GraphicsContext>,
) -> StrResult {
    let xr_session = Arc::new(RwLock::new(OpenxrSession::new(
        Arc::clone(&xr_context),
        Arc::clone(&graphics_context),
    )?));

    let mut scene = Scene::new(&graphics_context)?;

    let streaming_components = Arc::new(Mutex::new(None::<VideoStreamingComponents>));

    // todo: init async runtime and sockets

    // this is used to keep the last stream frame in place when the stream is stuck
    let old_stream_view_configs = vec![];

    loop {
        let xr_session_rlock = xr_session.read();
        let mut presentation_guard = if let Some(guard) = xr_session_rlock.begin_frame()? {
            guard
        } else {
            continue;
        };

        let maybe_stream_view_configs =
            video_streaming_pipeline(&streaming_components, &mut presentation_guard);
        presentation_guard.scene_view_configs =
            if let Some(stream_view_configs) = maybe_stream_view_configs.clone() {
                stream_view_configs
            } else {
                old_stream_view_configs.clone()
            };

        let scene_input = xr_session_rlock.get_scene_input()?;

        scene.update(
            scene_input.left_pose_input,
            scene_input.right_pose_input,
            scene_input.buttons,
            maybe_stream_view_configs.is_some(),
        );
        presentation_guard.scene_view_configs = scene_input.view_configs;

        for (index, acquired_swapchain) in presentation_guard
            .acquired_scene_swapchains
            .iter_mut()
            .enumerate()
        {
            scene.render(
                &presentation_guard.scene_view_configs[index],
                Arc::clone(&acquired_swapchain.texture_view),
                acquired_swapchain.size,
            )
        }
    }
}

// Returns true if stream is updated for the current frame
fn video_streaming_pipeline(
    streaming_components: &Arc<Mutex<Option<VideoStreamingComponents>>>,
    presentation_guard: &mut OpenxrPresentationGuard,
) -> Option<Vec<ViewConfig>> {
    if let Some(streaming_components) = streaming_components.lock().as_ref() {
        let decoder_target = streaming_components.compositor.input_texture();

        let timeout = Duration::from_micros(
            (presentation_guard.predicted_frame_interval.as_micros() as f32
                * FRAME_TIMEOUT_MULTIPLIER) as _,
        );
        let frame_metadata = get_video_frame_data(streaming_components, decoder_target, timeout)?;

        let compositor_target = presentation_guard
            .acquired_stream_swapchains
            .iter()
            .map(|swapchain| Arc::clone(&swapchain.texture_view))
            .collect::<Vec<_>>();

        streaming_components.compositor.render(&compositor_target);

        presentation_guard.display_timestamp = frame_metadata.timestamp;

        Some(frame_metadata.view_configs)
    } else {
        None
    }
}

// Dequeue decoded frames and metadata and makes sure they are on the same latest timestamp
fn get_video_frame_data(
    streaming_components: &VideoStreamingComponents,
    decoder_target: &Texture,
    timeout: Duration,
) -> Option<FrameMetadataPacket> {
    let mut frame_metadata = streaming_components
        .frame_metadata_receiver
        .recv_timeout(timeout)
        .ok()?;

    let mut decoder_timestamps = vec![];
    for decoder in &streaming_components.video_decoders {
        decoder_timestamps.push(
            decoder
                .get_output_frame(decoder_target, 0, timeout)
                .ok()
                .flatten()?,
        );
    }

    let greatest_timestamp = decoder_timestamps
        .iter()
        .cloned()
        .fold(frame_metadata.timestamp, Duration::max);

    while frame_metadata.timestamp < greatest_timestamp {
        frame_metadata = streaming_components
            .frame_metadata_receiver
            .recv_timeout(timeout)
            .ok()?;
    }

    for (mut timestamp, decoder) in decoder_timestamps
        .into_iter()
        .zip(streaming_components.video_decoders.iter())
    {
        while timestamp < greatest_timestamp {
            timestamp = decoder
                .get_output_frame(decoder_target, 0, timeout)
                .ok()
                .flatten()?;
        }
    }

    Some(frame_metadata)
}
