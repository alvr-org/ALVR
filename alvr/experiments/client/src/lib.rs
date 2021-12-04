mod connection;
mod scene;
mod streaming_compositor;
mod video_decoder;
mod xr;

use crate::xr::{XrContext, XrEvent, XrPresentationGuard, XrSession};
use alvr_common::{
    glam::{Quat, Vec3},
    log,
    prelude::*,
    Fov,
};
use alvr_graphics::GraphicsContext;
use connection::VideoFrameMetadataPacket;
use parking_lot::{Mutex, RwLock};
use scene::Scene;
use std::{sync::Arc, thread, time::Duration};
use streaming_compositor::StreamingCompositor;
use video_decoder::VideoDecoder;
use wgpu::Texture;

const MAX_SESSION_LOOP_FAILS: usize = 5;

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
    frame_metadata_receiver: crossbeam_channel::Receiver<VideoFrameMetadataPacket>,
}

#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {
    env_logger::init();
    log::error!("enter main");

    show_err(run());

    #[cfg(target_os = "android")]
    ndk_glue::native_activity().finish();
}

fn run() -> StrResult {
    let xr_context = Arc::new(XrContext::new());

    let graphics_context = Arc::new(xr::create_graphics_context(&xr_context)?);

    let mut fails_count = 0;
    loop {
        let res = show_err(session_pipeline(
            Arc::clone(&xr_context),
            Arc::clone(&graphics_context),
        ));

        if res.is_some() {
            break Ok(());
        } else {
            thread::sleep(Duration::from_millis(500));

            fails_count += 1;

            if fails_count == MAX_SESSION_LOOP_FAILS {
                log::error!("session loop failed {} times. Terminating.", fails_count);
                break Ok(());
            }
        }
    }
}

fn session_pipeline(
    xr_context: Arc<XrContext>,
    graphics_context: Arc<GraphicsContext>,
) -> StrResult {
    let xr_session = Arc::new(RwLock::new(XrSession::new(
        Arc::clone(&xr_context),
        Arc::clone(&graphics_context),
    )?));
    log::error!("session created");

    let mut scene = Scene::new(Arc::clone(&graphics_context))?;
    log::error!("scene created");

    let streaming_components = Arc::new(Mutex::new(None::<VideoStreamingComponents>));

    // todo: init async runtime and sockets

    // this is used to keep the last stream frame in place when the stream is stuck
    let old_stream_view_configs = vec![];

    loop {
        let xr_session_rlock = xr_session.read();
        let mut presentation_guard = match xr_session_rlock.begin_frame()? {
            XrEvent::ShouldRender(guard) => guard,
            XrEvent::Idle => continue,
            XrEvent::Shutdown => return Ok(()),
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
            scene_input.is_focused,
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
    presentation_guard: &mut XrPresentationGuard,
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
) -> Option<VideoFrameMetadataPacket> {
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
