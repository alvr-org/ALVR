use alvr_client_core::{ClientCapabilities, ClientCoreEvent};
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    parking_lot::RwLock,
    DeviceMotion, Pose, RelaxedAtomic, HEAD_ID,
};
use alvr_packets::Tracking;
use alvr_session::CodecType;
use eframe::{
    egui::{CentralPanel, Context, RichText, Slider, ViewportBuilder},
    Frame, NativeOptions,
};
use std::{
    f32::consts::{FRAC_PI_2, PI},
    sync::{
        mpsc::{self, TryRecvError},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq)]
struct WindowInput {
    height: f32,
    yaw: f32,
    pitch: f32,
    use_random_position: bool,
    random_position_offset_magnitude: f32,
    random_position_interval_ms: u64,
    emulated_decode_ms: u64,
    emulated_compositor_ms: u64,
    emulated_vsync_ms: u64,
}

impl Default for WindowInput {
    fn default() -> Self {
        Self {
            height: 1.5,
            yaw: 0.0,
            pitch: 0.0,
            use_random_position: true,
            random_position_offset_magnitude: 0.01,
            random_position_interval_ms: 2000,
            emulated_decode_ms: 5,
            emulated_compositor_ms: 1,
            emulated_vsync_ms: 25,
        }
    }
}

#[derive(Clone)]
struct WindowOutput {
    hud_message: String,
    fps: f32,
    connected: bool,
    resolution: UVec2,
    decoder_codec: Option<CodecType>,
    current_frame_timestamp: Duration,
}

impl Default for WindowOutput {
    fn default() -> Self {
        Self {
            hud_message: "".into(),
            fps: 60.0,
            connected: false,
            resolution: UVec2::ZERO,
            decoder_codec: None,
            current_frame_timestamp: Duration::ZERO,
        }
    }
}

pub struct Window {
    input: WindowInput,
    input_sender: mpsc::Sender<WindowInput>,
    output: WindowOutput,
    output_receiver: mpsc::Receiver<WindowOutput>,
}

impl Window {
    fn new(
        input_sender: mpsc::Sender<WindowInput>,
        output_receiver: mpsc::Receiver<WindowOutput>,
    ) -> Self {
        Self {
            input: WindowInput::default(),
            input_sender,
            output: WindowOutput::default(),
            output_receiver,
        }
    }
}

impl eframe::App for Window {
    fn update(&mut self, context: &Context, _: &mut Frame) {
        while let Ok(output) = self.output_receiver.try_recv() {
            self.output = output;
        }

        let mut input = self.input.clone();

        CentralPanel::default().show(context, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new(&self.output.hud_message));
            });
            ui.label(format!("FPS: {}", self.output.fps));
            ui.label(format!("Connected: {}", self.output.connected));
            ui.label(format!("View resolution: {}", self.output.resolution));
            ui.label(format!("Codec: {:?}", self.output.decoder_codec));
            ui.label(format!(
                "Current frame: {:?}",
                self.output.current_frame_timestamp
            ));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(Slider::new(&mut input.height, 0.0..=2.0));
            });
            ui.horizontal(|ui| {
                ui.label("Yaw:");
                ui.add(Slider::new(&mut input.yaw, -PI..=PI));
            });
            ui.horizontal(|ui| {
                ui.label("Pitch:");
                ui.add(Slider::new(&mut input.pitch, -FRAC_PI_2..=FRAC_PI_2));
            });
            ui.checkbox(&mut input.use_random_position, "Use random position");
            ui.horizontal(|ui| {
                ui.label("Random position offset magnitude:");
                ui.add(Slider::new(
                    &mut input.random_position_offset_magnitude,
                    0.0..=0.1,
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Random position interval ms");
                ui.add(Slider::new(
                    &mut input.random_position_interval_ms,
                    0..=10_000,
                ));
            });
        });

        if input != self.input {
            self.input = input;

            self.input_sender.send(self.input.clone()).ok();
        }

        context.request_repaint();
    }
}

fn tracking_thread(streaming: Arc<RelaxedAtomic>, fps: f32, input: Arc<RwLock<WindowInput>>) {
    let timestamp_origin = Instant::now();

    let mut position_offset = Vec3::ZERO;

    let mut loop_deadline = Instant::now();
    let mut random_position_deadline = Instant::now();
    while streaming.value() {
        let input_lock = input.read();

        let orientation =
            Quat::from_rotation_y(input_lock.yaw) * Quat::from_rotation_x(input_lock.pitch);

        if input_lock.use_random_position && Instant::now() > random_position_deadline {
            random_position_deadline =
                Instant::now() + Duration::from_millis(input_lock.random_position_interval_ms);

            position_offset = (Vec3::new(rand::random(), rand::random(), rand::random())
                - Vec3::ONE / 0.5)
                * input_lock.random_position_offset_magnitude;
        }

        let position = Vec3::new(0.0, input_lock.height, 0.0) + position_offset;

        alvr_client_core::send_tracking(Tracking {
            target_timestamp: Instant::now() - timestamp_origin
                + alvr_client_core::get_head_prediction_offset(),
            device_motions: vec![(
                *HEAD_ID,
                DeviceMotion {
                    pose: Pose {
                        orientation,
                        position,
                    },
                    linear_velocity: Vec3::ZERO,
                    angular_velocity: Vec3::ZERO,
                },
            )],
            ..Default::default()
        });

        drop(input_lock);

        loop_deadline += Duration::from_secs_f32(1.0 / fps / 3.0);
        thread::sleep(loop_deadline.saturating_duration_since(Instant::now()))
    }
}

fn client_thread(
    output_sender: mpsc::Sender<WindowOutput>,
    input_receiver: mpsc::Receiver<WindowInput>,
) {
    alvr_client_core::initialize(ClientCapabilities {
        default_view_resolution: UVec2::new(1920, 1832),
        external_decoder: true,
        refresh_rates: vec![60.0, 72.0, 80.0, 90.0, 120.0],
        foveated_encoding: false,
        encoder_high_profile: false,
        encoder_10_bits: false,
        encoder_av1: false,
    });
    alvr_client_core::resume();

    let streaming = Arc::new(RelaxedAtomic::new(true));
    let mut maybe_tracking_thread = None;

    let mut window_output = WindowOutput::default();
    let window_input = Arc::new(RwLock::new(WindowInput::default()));

    let mut deadline = Instant::now();
    'main_loop: loop {
        let input_lock = window_input.read();

        while let Some(event) = alvr_client_core::poll_event() {
            match event {
                ClientCoreEvent::UpdateHudMessage(message) => {
                    window_output.hud_message = message;
                }
                ClientCoreEvent::StreamingStarted {
                    negotiated_config, ..
                } => {
                    window_output.fps = negotiated_config.refresh_rate_hint;
                    window_output.connected = true;
                    window_output.resolution = negotiated_config.view_resolution;

                    let streaming = Arc::clone(&streaming);
                    let input = Arc::clone(&window_input);
                    maybe_tracking_thread = Some(thread::spawn(move || {
                        tracking_thread(streaming, negotiated_config.refresh_rate_hint, input)
                    }));
                }
                ClientCoreEvent::StreamingStopped => {
                    window_output.connected = true;
                    if let Some(thread) = maybe_tracking_thread.take() {
                        thread.join().ok();
                    }
                }
                ClientCoreEvent::Haptics { .. } => (),
                ClientCoreEvent::DecoderConfig { codec, .. } => {
                    window_output.decoder_codec = Some(codec)
                }
                ClientCoreEvent::FrameReady { timestamp, .. } => {
                    window_output.current_frame_timestamp = timestamp;

                    thread::sleep(Duration::from_millis(input_lock.emulated_decode_ms));
                    alvr_client_core::report_frame_decoded(timestamp);
                }
            }

            output_sender.send(window_output.clone()).ok();
        }

        thread::sleep(Duration::from_millis(3));

        alvr_client_core::report_compositor_start(window_output.current_frame_timestamp);

        thread::sleep(Duration::from_millis(input_lock.emulated_compositor_ms));

        alvr_client_core::report_submit(
            window_output.current_frame_timestamp,
            Duration::from_millis(input_lock.emulated_vsync_ms),
        );

        drop(input_lock);

        match input_receiver.try_recv() {
            Ok(input) => *window_input.write() = input,
            Err(TryRecvError::Disconnected) => break 'main_loop,
            Err(TryRecvError::Empty) => (),
        }

        deadline += Duration::from_secs_f32(1.0 / window_output.fps);
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }

    streaming.set(false);
    if let Some(thread) = maybe_tracking_thread {
        thread.join().unwrap();
    }

    alvr_client_core::pause();
    alvr_client_core::destroy();
}

fn main() {
    env_logger::init();

    let (input_sender, input_receiver) = mpsc::channel::<WindowInput>();
    let (output_sender, output_receiver) = mpsc::channel::<WindowOutput>();

    let client_thread = thread::spawn(|| {
        client_thread(output_sender, input_receiver);
    });

    eframe::run_native(
        "Mock client",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size((400.0, 400.0)),
            ..Default::default()
        },
        Box::new(|_| Box::new(Window::new(input_sender, output_receiver))),
    )
    .ok();

    client_thread.join().unwrap();
}
