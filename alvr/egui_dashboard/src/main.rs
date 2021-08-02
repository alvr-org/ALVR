use alvr_common::data::SessionManager;
use alvr_gui::{
    dashboard::{Dashboard, DashboardResponse},
    translation::TranslationBundle,
};
use eframe::{
    egui::CtxRef,
    epi::{self, Frame, Storage},
};
use std::{env, fs, sync::Arc};

fn get_translation_bundle(locale: &str) -> Arc<TranslationBundle> {
    let exe_path = env::current_exe().unwrap();
    let dir = exe_path.parent().unwrap();

    let locale = if locale.is_empty() {
        None
    } else {
        Some(locale.to_owned())
    };

    Arc::new(
        TranslationBundle::new(
            locale,
            &fs::read_to_string(dir.join("languages").join("list.json")).unwrap(),
            |language_id| {
                fs::read_to_string(dir.join("languages").join(format!("{}.ftl", language_id)))
                    .unwrap()
            },
        )
        .unwrap(),
    )
}

struct App {
    dashboard: Dashboard,
    session_manager: SessionManager,
    last_locale: String,
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

                    if session.locale != self.last_locale {
                        self.last_locale = session.locale.clone();

                        self.dashboard =
                            Dashboard::new(&session, get_translation_bundle(&session.locale));
                    }

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
    let last_locale = session_manager.get().locale.clone();

    let app = App {
        dashboard: Dashboard::new(session_manager.get(), get_translation_bundle(&last_locale)),
        session_manager,
        last_locale,
    };

    eframe::run_native(Box::new(app), options)
}
