use alvr_common::Fov;
use alvr_ipc::{DriverRequest, Layer, ResponseForDriver, SsePacket, VideoConfigUpdate};
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
    println!("Waiting for connection...");

    let (mut ipc_client, mut sse_receiver) =
        alvr_ipc::ipc_connect("/tmp/alvr_driver_request.sock", "/tmp/alvr_driver_sse.sock")
            .unwrap();

    println!("Server connected");

    let ipc_running = Arc::new(AtomicBool::new(true));

    println!("Requesting initialization config...");
    let presentation = if let ResponseForDriver::InitializationConfig {
        tracked_devices,
        presentation,
    } = ipc_client
        .request(&DriverRequest::GetInitializationConfig)
        .unwrap()
    {
        dbg!(tracked_devices);
        presentation
    } else {
        unreachable!()
    };

    let video_config = Arc::new(Mutex::new(VideoConfigUpdate {
        preferred_view_size: (500, 500),
        fov: [Fov::default(), Fov::default()],
        ipd_m: 0.65,
        fps: 60.0,
    }));

    thread::spawn({
        let ipc_running = Arc::clone(&ipc_running);
        let video_config = Arc::clone(&video_config);
        move || {
            while ipc_running.load(Ordering::Relaxed) {
                if let Ok(maybe_message) = sse_receiver.receive_non_blocking() {
                    match maybe_message {
                        Some(SsePacket::Restart) => {
                            println!("Restart requested. todo: restart");
                            ipc_running.store(false, Ordering::Relaxed);
                            // todo: implement restart
                        }
                        Some(SsePacket::UpdateVideoConfig(config)) => {
                            println!("Video config updated: {:?}", &config);
                            *video_config.lock().unwrap() = config;
                        }
                        Some(_) => println!("Ignored server packet"),
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

    if presentation {
        println!("Presentation enabled. Requesting a swapchain");
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
            println!("Get next swapchain index");
            let idx = if let ResponseForDriver::SwapchainIndex(idx) = ipc_client
                .request(&DriverRequest::GetNextSwapchainIndex { id: swapchain_id })
                .unwrap()
            {
                idx
            } else {
                unreachable!()
            };

            println!("Rendering");
            // todo: render something here
            // texture[idx];

            thread::sleep(
                Instant::now() - presentation_time
                    + Duration::from_secs_f32(1.0 / video_config.lock().unwrap().fps),
            );

            println!("Present layers");
            presentation_time = Instant::now();
            ipc_client
                .request(&DriverRequest::PresentLayers(vec![vec![
                    Layer {
                        orientation: Default::default(),
                        fov: Default::default(),
                        swapchain_id,
                        rect_offset: (0.0, 0.0),
                        rect_size: (960.0, 1080.0),
                    },
                    Layer {
                        orientation: Default::default(),
                        fov: Default::default(),
                        swapchain_id, // use same swapchain for both eyes for now
                        rect_offset: (0.0, 0.0),
                        rect_size: (960.0, 1080.0),
                    },
                ]]))
                .unwrap();
        }
    } else {
        println!("Presentation disabled. Idle");
        while ipc_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(500));
        }
    }
}
