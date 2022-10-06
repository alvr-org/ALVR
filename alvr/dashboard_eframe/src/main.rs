use std::{env, fs, path::PathBuf, sync::Arc};

struct ALVRDashboard {
    dashboard: dashboard_core::dashboard::Dashboard,
    counter: u32,
    last_vals: (f32, f32),
}

impl ALVRDashboard {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let dir = PathBuf::from(env::var("DIR").unwrap());

        Self {
            dashboard: dashboard_core::dashboard::Dashboard::new(
                &alvr_session::SessionDesc::default(),
                Arc::new(
                    dashboard_core::translation::TranslationBundle::new(
                        Some("en".to_string()),
                        &std::fs::read_to_string(dir.join("languages").join("list.json")).unwrap(),
                        |language_id| {
                            fs::read_to_string(
                                dir.join("languages").join(format!("{}.ftl", language_id)),
                            )
                            .unwrap()
                        },
                    )
                    .unwrap(),
                ),
            ),
            counter: 0,
            last_vals: (0.0, 0.0),
        }
    }
}

impl eframe::App for ALVRDashboard {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.counter += 1;
        if self.counter == 100 {
            self.last_vals = (rand::random::<f32>() * 50.0, rand::random::<f32>() * 50.0);
            self.counter = 0;
        }
        self.dashboard.update(
            ctx,
            &alvr_session::SessionDesc::default(),
            &[alvr_events::Event {
                timestamp: "".to_string(),
                event_type: alvr_events::EventType::GraphStatistics(alvr_events::GraphStatistics {
                    total_pipeline_latency_s: 12.0 + self.last_vals.0 / 5.0 + self.last_vals.0,
                    game_time_s: self.last_vals.0,
                    server_compositor_s: self.last_vals.0 / 10.0,
                    encoder_s: 5.0,
                    network_s: 2.0,
                    decoder_s: 5.0,
                    client_compositor_s: self.last_vals.0 / 10.0,
                    vsync_queue_s: 0.0,
                    client_fps: self.last_vals.0,
                    server_fps: self.last_vals.1,
                }),
            }],
        );
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ALVR Dashboard",
        native_options,
        Box::new(|cc| Box::new(ALVRDashboard::new(cc))),
    );
}
