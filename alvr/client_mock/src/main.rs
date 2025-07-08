use alvr_client_core::{ClientCapabilities, ClientCoreContext, ClientCoreEvent};
use alvr_common::{
    DeviceMotion, HEAD_ID, Pose, RelaxedAtomic, ViewParams,
    glam::{Quat, UVec2, Vec3},
    parking_lot::RwLock,
};
use alvr_packets::TrackingData;
use alvr_session::CodecType;
use eframe::{
    Frame, NativeOptions,
    egui::{CentralPanel, Context, RichText, Slider, ViewportBuilder},
};
use std::{
    f32::consts::{FRAC_PI_2, PI},
    sync::{
        Arc,
        mpsc::{self, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq)]
struct WindowInput {
    height: f32,
    yaw: f32,
    pitch: f32,
    use_random_orientation: bool,
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
            use_random_orientation: true,
            emulated_decode_ms: 5,
            emulated_compositor_ms: 1,
            emulated_vsync_ms: 25,
        }
    }
}

#[derive(Clone)]
struct WindowOutput {
    hud_message: String,
    connected: bool,
    fps: f32,
    resolution: UVec2,
    decoder_codec: Option<CodecType>,
    current_frame_timestamp: Duration,
}

impl Default for WindowOutput {
    fn default() -> Self {
        Self {
            hud_message: "".into(),
            connected: false,
            fps: 1.0,
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
            ui.label(format!("Connected: {}", self.output.connected));
            ui.label(format!("FPS: {}", self.output.fps));
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
            ui.checkbox(
                &mut input.use_random_orientation,
                "Use randomized orientation offset",
            );
        });

        if input != self.input {
            self.input = input;

            self.input_sender.send(self.input.clone()).ok();
        }

        context.request_repaint();
    }
}

fn tracking_thread(
    context: Arc<ClientCoreContext>,
    streaming: Arc<RelaxedAtomic>,
    fps: f32,
    input: Arc<RwLock<WindowInput>>,
) {
    let timestamp_origin = Instant::now();
    context.send_view_params([ViewParams::DUMMY; 2]);

    let mut loop_deadline = Instant::now();
    while streaming.value() {
        let input_lock = input.read();

        let mut orientation =
            Quat::from_rotation_y(input_lock.yaw) * Quat::from_rotation_x(input_lock.pitch);

        if input_lock.use_random_orientation {
            orientation *= Quat::from_rotation_z(rand::random::<f32>() * 0.001);
        }

        let position = Vec3::new(0.0, input_lock.height, 0.0);

        context.send_tracking(TrackingData {
            poll_timestamp: timestamp_origin.elapsed(),
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
            hand_skeletons: [None, None],
            face_data: Default::default(),
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
    let capabilities = ClientCapabilities {
        default_view_resolution: UVec2::new(1920, 1832),
        refresh_rates: vec![60.0, 72.0, 80.0, 90.0, 120.0],
        foveated_encoding: false,
        encoder_high_profile: false,
        encoder_10_bits: false,
        encoder_av1: false,
        prefer_10bit: false,
        prefer_full_range: true,
        preferred_encoding_gamma: 1.0,
        prefer_hdr: false,
    };
    let client_core_context = Arc::new(ClientCoreContext::new(capabilities));

    client_core_context.resume();

    let streaming = Arc::new(RelaxedAtomic::new(false));
    let got_decoder_config = Arc::new(RelaxedAtomic::new(false));
    let mut maybe_tracking_thread = None;

    let mut window_output = WindowOutput::default();
    let window_input = Arc::new(RwLock::new(WindowInput::default()));

    let mut deadline = Instant::now();
    'main_loop: loop {
        let input_lock = window_input.read();

        while let Some(event) = client_core_context.poll_event() {
            match event {
                ClientCoreEvent::UpdateHudMessage(message) => {
                    window_output.hud_message = message;
                }
                ClientCoreEvent::StreamingStarted(config) => {
                    window_output.fps = config.negotiated_config.refresh_rate_hint;
                    window_output.connected = true;
                    window_output.resolution = config.negotiated_config.view_resolution;

                    streaming.set(true);

                    let context = Arc::clone(&client_core_context);
                    let streaming = Arc::clone(&streaming);
                    let input = Arc::clone(&window_input);
                    maybe_tracking_thread = Some(thread::spawn(move || {
                        tracking_thread(
                            context,
                            streaming,
                            config.negotiated_config.refresh_rate_hint,
                            input,
                        )
                    }));
                }
                ClientCoreEvent::StreamingStopped => {
                    streaming.set(false);
                    got_decoder_config.set(false);

                    if let Some(thread) = maybe_tracking_thread.take() {
                        thread.join().ok();
                    }

                    window_output.fps = 1.0;
                    window_output.connected = false;
                    window_output.resolution = UVec2::ZERO;
                    window_output.decoder_codec = None;
                }
                ClientCoreEvent::DecoderConfig { codec, .. } => {
                    got_decoder_config.set(true);

                    window_output.decoder_codec = Some(codec);
                }
                ClientCoreEvent::Haptics { .. } | ClientCoreEvent::RealTimeConfig(_) => (),
            }

            output_sender.send(window_output.clone()).ok();
        }

        thread::sleep(Duration::from_millis(3));

        client_core_context.report_compositor_start(window_output.current_frame_timestamp);

        thread::sleep(Duration::from_millis(input_lock.emulated_compositor_ms));

        client_core_context.report_submit(
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

    client_core_context.pause()

    // client_core_context destroy is called here on drop
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
        Box::new(|_| Ok(Box::new(Window::new(input_sender, output_receiver)))),
    )
    .ok();

    client_thread.join().unwrap();
}
