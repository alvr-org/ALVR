use alvr_common::dashboard::Dashboard;
use eframe::{egui, epi};

#[derive(Default)]
struct App {
    dashboard: Dashboard,
}

impl epi::App for App {
    fn name(&self) -> &str {
        "ALVR Dashboard"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _: &mut epi::Frame<'_>) {
        self.dashboard.draw(ctx);
    }
}

fn main() {
    let options = eframe::NativeOptions::default();

    eframe::run_native(Box::new(App::default()), options)
}
