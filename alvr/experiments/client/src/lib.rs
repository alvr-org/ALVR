mod connection;
mod openxr;
mod scene;
mod streaming_compositor;
mod video_decoder;

use crate::openxr::{OpenxrContext, OpenxrSession};
use alvr_common::prelude::*;
use alvr_graphics::GraphicsContext;
use connection::VideoPacket;
use parking_lot::Mutex;
use scene::SceneRenderer;
use std::{sync::Arc, time::Duration};
use streaming_compositor::StreamingCompositor;
use video_decoder::VideoDecoder;

struct VideoStreamingComponents {
    compositor: StreamingCompositor,
    video_decoder: Vec<VideoDecoder>,
    video_stream_receivers: Vec<crossbeam_channel::Receiver<VideoPacket>>,
}

fn session_pipeline(
    xr_context: Arc<OpenxrContext>,
    graphics_context: Arc<GraphicsContext>,
) -> StrResult {
    let xr_session = Arc::new(OpenxrSession::new(
        Arc::clone(&xr_context),
        Arc::clone(&graphics_context),
    )?);

    let mut scene = SceneRenderer::new(&graphics_context)?;

    let streaming_components = Arc::new(Mutex::new(None::<VideoStreamingComponents>));

    // todo: init async runtime and sockets

    loop {
        let session_lock = if let Some(lock) = xr_session.begin_frame()? {
            lock
        } else {
            return Ok(());
        };

        let display_time;
        if let Some(streaming_components) = streaming_components.lock().as_ref() {
            //todo: decode, compose frames

            display_time = todo!();
        } else {
            display_time = Duration::from_nanos(
                session_lock.frame_state.predicted_display_time.as_nanos() as _,
            );
        }

        // todo: get poses with display_time, render scene

        xr_session.end_frame(display_time, vec![], vec![])?;
    }
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

#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {
    show_err(run());

    #[cfg(target_os = "android")]
    ndk_glue::native_activity().finish();
}
