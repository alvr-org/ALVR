#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod commands;

use alvr_common::prelude::*;
use alvr_filesystem as afs;
use eframe::{
    egui::{self, RichText},
    epaint::Vec2,
    Theme,
};
use std::{
    env,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const WINDOW_WIDTH: f32 = 500.0;
const WINDOW_HEIGHT: f32 = 300.0;

const FONT_SIZE: f32 = 20.0;

#[derive(Clone)]
enum View {
    RequirementsCheck { steamvr: String },
    Launching { resetting: bool },
}

struct State {
    view: View,
}

struct ALVRLauncher {
    state: Arc<Mutex<State>>,
}

impl ALVRLauncher {
    fn new(_cc: &eframe::CreationContext<'_>, state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

fn launcher_lifecycle(state: Arc<Mutex<State>>) {
    loop {
        let steamvr_ok = commands::check_steamvr_installation();

        if steamvr_ok {
            break;
        } else {
            let steamvr_string =
                "SteamVR not installed: make sure you launched it at least once, then close it.";

            state.lock().unwrap().view = View::RequirementsCheck {
                steamvr: steamvr_string.to_owned(),
            };

            thread::sleep(Duration::from_millis(500));
        }
    }

    state.lock().unwrap().view = View::Launching { resetting: false };

    let request_agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(100))
        .build();

    let mut tried_steamvr_launch = false;
    loop {
        // get a small non-code file
        let maybe_response = request_agent.get("http://127.0.0.1:8082/index.html").call();
        if let Ok(response) = maybe_response {
            if response.status() == 200 {
                std::process::exit(0);
            }
        }

        // try to launch SteamVR only one time automatically
        if !tried_steamvr_launch {
            if alvr_common::show_err(commands::maybe_register_alvr_driver()).is_some() {
                if commands::is_steamvr_running() {
                    commands::kill_steamvr();
                    thread::sleep(Duration::from_secs(2))
                }
                commands::maybe_launch_steamvr();
            }
            tried_steamvr_launch = true;
        }

        thread::sleep(Duration::from_millis(500));
    }
}

fn reset_and_retry(state: Arc<Mutex<State>>) {
    thread::spawn(move || {
        state.lock().unwrap().view = View::Launching { resetting: true };

        commands::kill_steamvr();
        commands::fix_steamvr();
        commands::restart_steamvr();

        thread::sleep(Duration::from_secs(2));

        state.lock().unwrap().view = View::Launching { resetting: false };
    });
}

fn text(text: impl Into<String>) -> RichText {
    RichText::new(text).size(FONT_SIZE)
}

impl eframe::App for ALVRLauncher {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| match &self.state.lock().unwrap().view.clone() {
                View::RequirementsCheck { steamvr } => {
                    ui.add_space(60.0);
                    ui.label(text(steamvr.clone()));
                }
                View::Launching { resetting } => {
                    ui.add_space(60.0);
                    ui.label(text("Waiting for server to load...").size(25.0));
                    ui.add_space(15.0);
                    if !resetting {
                        if ui.button(text("Reset drivers and retry")).clicked() {
                            reset_and_retry(Arc::clone(&self.state));
                        }
                    } else {
                        ui.label(text("Please wait for multiple restarts"));
                    }
                }
            });
        });
    }
}

fn make_window() -> StrResult {
    env_logger::init();

    let instance_mutex =
        single_instance::SingleInstance::new("alvr_launcher_mutex").map_err(err!())?;
    if instance_mutex.is_single() {
        let driver_dir = afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
            .openvr_driver_root_dir;

        if driver_dir.to_str().filter(|s| s.is_ascii()).is_none() {
            alvr_common::show_e_blocking(format!(
                "The path of this folder ({}) contains non ASCII characters. {}",
                driver_dir.to_string_lossy(),
                "Please move it somewhere else (for example in C:\\Users\\Public\\Documents).",
            ));
            return Ok(());
        }

        let state = Arc::new(Mutex::new(State {
            view: View::RequirementsCheck { steamvr: "".into() },
        }));

        thread::spawn({
            let state = Arc::clone(&state);
            move || launcher_lifecycle(state)
        });

        eframe::run_native(
            "ALVR Launcher",
            eframe::NativeOptions {
                centered: true,
                initial_window_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
                resizable: false,
                default_theme: Theme::Light,
                ..Default::default()
            },
            Box::new(|cc| Box::new(ALVRLauncher::new(cc, state))),
        );
    }
    Ok(())
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    match args.get(1) {
        Some(flag) if flag == "--restart-steamvr" => commands::restart_steamvr(),
        Some(flag) if flag == "--update" => commands::invoke_installer(),
        Some(_) | None => {
            alvr_common::show_err_blocking(make_window());
        }
    }
}
