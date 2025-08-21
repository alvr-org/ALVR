use alvr_common::{
    HEAD_ID, Pose, RelaxedAtomic, ViewParams, error, info,
    parking_lot::{Mutex, RwLock},
};
use alvr_filesystem as afs;
use alvr_server_core::{ServerCoreContext, ServerCoreEvent};
use alvr_session::CodecType;
use mp4::MediaType;
use std::{
    env,
    fs::File,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

fn main() {
    let filesystem_layout = afs::Layout::new(env::current_exe().unwrap().parent().unwrap());
    alvr_server_core::initialize_environment(filesystem_layout.clone());

    let log_to_disk = alvr_server_core::settings().extra.logging.log_to_disk;

    alvr_server_core::init_logging(
        log_to_disk.then_some(filesystem_layout.session_log()),
        Some(filesystem_layout.crash_log()),
    );

    let (context, events_receiver) = ServerCoreContext::new();

    let mut video_thread = None;
    let is_client_connected = Arc::new(RelaxedAtomic::new(false));
    let last_pose_timestamp = Arc::new(Mutex::new(Duration::ZERO));
    let last_head_pose = Arc::new(Mutex::new(Pose::default()));
    let local_view_params = Arc::new(RwLock::new([ViewParams::DUMMY; 2]));

    context.start_connection();

    let context = Arc::new(RwLock::new(Some(context)));

    loop {
        let event = match events_receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        match event {
            ServerCoreEvent::SetOpenvrProperty { .. } => (),
            ServerCoreEvent::ClientConnected => {
                is_client_connected.set(true);

                let video_path = filesystem_layout
                    .static_resources_dir
                    .join("test_video.mp4");

                let context = Arc::clone(&context);
                let is_client_connected = Arc::clone(&is_client_connected);
                let last_pose_timestamp = Arc::clone(&last_pose_timestamp);
                let last_head_pose = Arc::clone(&last_head_pose);
                let local_view_params = Arc::clone(&local_view_params);

                video_thread = Some(thread::spawn(move || {
                    let video_file = File::open(video_path).unwrap();

                    let mut mp4 = mp4::read_mp4(video_file).unwrap();

                    let Some(h264_track) = mp4
                        .tracks()
                        .values()
                        .find(|track| track.media_type().unwrap() == MediaType::H264)
                    else {
                        // Note: the crate mp4 provides only SPS and PPS. the HEVC VPS might be
                        // inside the stream but I haven't checked
                        error!("The video does not contain a H264 track");
                        return;
                    };

                    if let Some(context) = &*context.read() {
                        let mut config_nals = h264_track.sequence_parameter_set().unwrap().to_vec();
                        config_nals.extend(h264_track.picture_parameter_set().unwrap());

                        context.set_video_config_nals(config_nals, CodecType::H264);
                    }

                    let track_id = h264_track.track_id();
                    let sample_count = h264_track.sample_count();
                    let frametime = Duration::from_secs_f64(1.0 / h264_track.frame_rate());

                    info!(
                        "Video: track {track_id}, sample count {sample_count}, frame time {frametime:?}"
                    );

                    let mut deadline = Instant::now() + frametime;
                    let mut sample_id = 1;
                    while is_client_connected.value()
                        && let Some(context) = &*context.read()
                    {
                        let head_pose = *last_head_pose.lock();
                        let timestamp = *last_pose_timestamp.lock();

                        info!("Reading video sample {sample_id}");
                        let sample = mp4.read_sample(track_id, sample_id).unwrap().unwrap();

                        context.report_composed(timestamp, Duration::ZERO);
                        context.report_present(timestamp, Duration::ZERO);

                        let local_views_params = local_view_params.read();

                        let global_view_params = [
                            ViewParams {
                                pose: head_pose * local_views_params[0].pose,
                                fov: local_views_params[0].fov,
                            },
                            ViewParams {
                                pose: head_pose * local_views_params[1].pose,
                                fov: local_views_params[1].fov,
                            },
                        ];

                        context.send_video_nal(
                            timestamp,
                            global_view_params,
                            true,
                            sample.bytes.to_vec(),
                        );

                        thread::sleep(
                            deadline
                                .checked_duration_since(Instant::now())
                                .unwrap_or(Duration::ZERO),
                        );
                        deadline += frametime;

                        sample_id += 1;
                        if sample_id >= sample_count {
                            sample_id = 1;
                        }
                    }
                }));
            }
            ServerCoreEvent::ClientDisconnected => {
                is_client_connected.set(false);
                if let Some(thread) = video_thread.take() {
                    thread.join().ok();
                }
            }
            ServerCoreEvent::Battery(_) => (),
            ServerCoreEvent::PlayspaceSync(_) => (),
            ServerCoreEvent::LocalViewParams(params) => *local_view_params.write() = params,
            ServerCoreEvent::Tracking { poll_timestamp } => {
                if let Some(context) = &*context.read()
                    && let Some(motion) = context.get_device_motion(*HEAD_ID, poll_timestamp)
                {
                    *last_pose_timestamp.lock() = poll_timestamp;

                    let motion = motion.predict(poll_timestamp, poll_timestamp);
                    *last_head_pose.lock() = motion.pose;
                }
            }
            ServerCoreEvent::Buttons(_) => (),
            ServerCoreEvent::RequestIDR => (),
            ServerCoreEvent::CaptureFrame => (),
            ServerCoreEvent::GameRenderLatencyFeedback(_) => (),
            ServerCoreEvent::ShutdownPending => break,
            ServerCoreEvent::RestartPending => {
                if let Some(context) = context.write().take() {
                    context.restart();
                }
                break;
            }
        }
    }
}
