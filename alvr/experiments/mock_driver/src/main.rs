use alvr_common::glam::Vec2;
use alvr_ipc::{DriverRequest, Layer, ResponseForDriver, SsePacket};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const VK_FORMAT_R8G8B8A8_SRGB: u32 = 43;
const DXGI_FORMAT_R8G8B8A8_UNORM_SRGB: u32 = 29;

fn main() {
    let (mut ipc_client, mut sse_receiver) = alvr_ipc::ipc_connect("driver").unwrap();

    let ipc_running = Arc::new(AtomicBool::new(true));

    let display_config = if let ResponseForDriver::InitializationConfig {
        tracked_devices: _,
        display_config,
    } = ipc_client
        .request(&DriverRequest::GetInitializationConfig)
        .unwrap()
    {
        display_config
    } else {
        unreachable!()
    };

    let display_config = display_config.unwrap(); // for now, there must always be a HMD
    let display_config = Arc::new(Mutex::new(display_config));

    thread::spawn({
        let ipc_running = Arc::clone(&ipc_running);
        let display_config = Arc::clone(&display_config);
        move || {
            while ipc_running.load(Ordering::Relaxed) {
                if let Ok(maybe_message) = sse_receiver.receive_non_blocking() {
                    match maybe_message {
                        Some(SsePacket::Restart) => {
                            ipc_running.store(false, Ordering::Relaxed);
                            // todo: implement restart
                        }
                        Some(SsePacket::UpdateVideoConfig(video_config)) => {
                            display_config.lock().unwrap().config = video_config;
                        }
                        Some(_) => (),
                        None => thread::sleep(Duration::from_millis(2)),
                    }
                } else {
                    ipc_running.store(false, Ordering::Relaxed)
                }
            }
        }
    });

    // this is used to close the sse thread once the main program flow has exited (forced or not)
    struct DropGuard {
        running: Arc<AtomicBool>,
    }

    impl Drop for DropGuard {
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
        }
    }

    let _drop_guard = DropGuard {
        running: Arc::clone(&ipc_running),
    };

    if display_config.lock().unwrap().presentation {
        let (swapchain_id, texture_handles) = if let ResponseForDriver::Swapchain { id, textures } =
            ipc_client
                .request(&DriverRequest::CreateSwapchain {
                    images_count: 3,
                    width: 960,
                    height: 1080,
                    format: if cfg!(windows) {
                        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
                    } else {
                        VK_FORMAT_R8G8B8A8_SRGB
                    },
                    sample_count: 1,
                })
                .unwrap()
        {
            (id, textures)
        } else {
            unreachable!()
        };

        // todo: get textures from handles

        let mut presentation_time = Instant::now();

        while ipc_running.load(Ordering::Relaxed) {
            let idx = if let ResponseForDriver::SwapchainIndex(idx) = ipc_client
                .request(&DriverRequest::GetNextSwapchainIndex { id: swapchain_id })
                .unwrap()
            {
                idx
            } else {
                unreachable!()
            };

            // todo: render something here
            // texture[idx];

            thread::sleep(
                Instant::now() - presentation_time
                    + Duration::from_secs_f32(1.0 / display_config.lock().unwrap().config.fps),
            );

            presentation_time = Instant::now();
            ipc_client
                .request(&DriverRequest::PresentLayers(vec![vec![
                    Layer {
                        orientation: Default::default(),
                        fov: Default::default(),
                        swapchain_id,
                        rect_offset: Vec2::new(0_f32, 0_f32),
                        rect_size: Vec2::new(960_f32, 1080_f32),
                    },
                    Layer {
                        orientation: Default::default(),
                        fov: Default::default(),
                        swapchain_id, // use same swapchain for both eyes for now
                        rect_offset: Vec2::new(0_f32, 0_f32),
                        rect_size: Vec2::new(960_f32, 1080_f32),
                    },
                ]]))
                .unwrap();
        }
    } else {
        while ipc_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(500));
        }
    }
}
