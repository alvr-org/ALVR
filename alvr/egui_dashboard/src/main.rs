use std::env;

use alvr_common::{
    dashboard::{Dashboard, DashboardResponse},
    data::{SessionManager, Theme},
};
use eframe::{
    egui::{CtxRef, Visuals},
    epi::{self, Frame, Storage},
};

struct App {
    dashboard: Dashboard,
    session_manager: SessionManager,
}

impl epi::App for App {
    fn name(&self) -> &str {
        "ALVR Dashboard"
    }

    fn setup(&mut self, ctx: &CtxRef, _: &mut Frame<'_>, _: Option<&dyn Storage>) {
        self.dashboard.setup(ctx);
    }

    fn update(&mut self, ctx: &CtxRef, _: &mut Frame<'_>) {
        if let Some(response) = self.dashboard.update(ctx, self.session_manager.get(), &[]) {
            match response {
                DashboardResponse::Connections(_) => todo!(),
                DashboardResponse::SessionUpdated(session) => {
                    println!("saving session");
                    *self.session_manager.get_mut() = *session;
                }
                DashboardResponse::PresetInvocation(_) => todo!(),
                DashboardResponse::Driver(_) => todo!(),
                DashboardResponse::FirewallRules(_) => todo!(),
                DashboardResponse::RestartSteamVR => todo!(),
                DashboardResponse::UpdateServer { url } => todo!(),
            }
        }
    }
}

fn main() {
    let options = eframe::NativeOptions::default();

    let session_manager = SessionManager::new(env::current_exe().unwrap().parent().unwrap());

    let app = App {
        dashboard: Dashboard::new(session_manager.get()),
        session_manager,
    };

    eframe::run_native(Box::new(app), options)
}
