#![windows_subsystem = "windows"]

mod commands;

use alvr_common::{
    prelude::*, send_launcher_packet, LauncherMessages, LauncherPacket, ALVR_LAUNCHER_ADDRESS,
};
use alvr_filesystem as afs;
use druid::{
    commands::CLOSE_WINDOW,
    theme,
    widget::{Button, CrossAxisAlignment, Flex, FlexParams, Label, LineBreaking, ViewSwitcher},
    AppDelegate, AppLauncher, Color, Command, Data, DelegateCtx, Env, ExtEventSink, FontDescriptor,
    Handled, Screen, Selector, Target, Widget, WindowDesc, WindowId,
};
use std::{
    env,
    io::{self, prelude::*, BufReader},
    net::TcpListener,
    process, thread,
    time::Duration,
};

use crate::commands::is_steamvr_running;

const WINDOW_WIDTH: f64 = 500.0;
const WINDOW_HEIGHT: f64 = 300.0;

const CHANGE_VIEW_CMD: Selector<View> = Selector::new("change_view");

#[derive(Clone, PartialEq, Data)]
enum View {
    RequirementsCheck { steamvr: String },
    Launching { resetting: bool },
}

fn launcher_lifecycle(handle: ExtEventSink, window_id: WindowId) {
    loop {
        let steamvr_ok = commands::check_steamvr_installation();

        if steamvr_ok {
            break;
        } else {
            let steamvr_string =
                "SteamVR not installed: make sure you launched it at least once, then close it.";

            handle
                .submit_command(
                    CHANGE_VIEW_CMD,
                    View::RequirementsCheck {
                        steamvr: steamvr_string.to_owned(),
                    },
                    Target::Auto,
                )
                .ok();

            thread::sleep(Duration::from_millis(500));
        }
    }

    handle
        .submit_command(
            CHANGE_VIEW_CMD,
            View::Launching { resetting: false },
            Target::Auto,
        )
        .ok();

    let request_agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(100))
        .build();

    let mut tried_steamvr_launch = false;
    loop {
        // get a small non-code file
        let maybe_response = request_agent.get("http://127.0.0.1:8082/index.html").call();
        if let Ok(response) = maybe_response {
            if response.status() == 200 {
                handle.submit_command(CLOSE_WINDOW, (), window_id).ok();
                break;
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

fn reset_and_retry(handle: ExtEventSink) {
    thread::spawn(move || {
        handle
            .submit_command(
                CHANGE_VIEW_CMD,
                View::Launching { resetting: true },
                Target::Auto,
            )
            .ok();

        commands::kill_steamvr();

        commands::fix_steamvr();

        commands::restart_steamvr();

        thread::sleep(Duration::from_secs(2));

        handle
            .submit_command(
                CHANGE_VIEW_CMD,
                View::Launching { resetting: false },
                Target::Auto,
            )
            .ok();
    });
}

fn gui() -> impl Widget<View> {
    ViewSwitcher::new(
        |view: &View, _| view.clone(),
        |view, _, _| match view {
            View::RequirementsCheck { steamvr } => Box::new(
                Flex::row()
                    .with_default_spacer()
                    .with_flex_child(
                        Flex::column()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .with_flex_spacer(1.0)
                            .with_child(
                                Label::new(steamvr.clone())
                                    .with_line_break_mode(LineBreaking::WordWrap),
                            )
                            .with_default_spacer()
                            .with_flex_spacer(1.5),
                        FlexParams::new(1.0, None),
                    )
                    .with_default_spacer(),
            ),
            View::Launching { resetting } => {
                let mut flex = Flex::column()
                    .with_spacer(60.0)
                    .with_child(Label::new("Waiting for server to load...").with_text_size(25.0))
                    .with_default_spacer();
                if !resetting {
                    flex = flex.with_child(
                        Button::new("Reset drivers and retry")
                            .on_click(move |ctx, _, _| reset_and_retry(ctx.get_external_handle())),
                    )
                } else {
                    flex = flex.with_child(Label::new("Please wait for multiple restarts"))
                }

                Box::new(flex.with_flex_spacer(1.0))
            }
        },
    )
}

struct Delegate;

impl AppDelegate<View> for Delegate {
    fn command(
        &mut self,
        _: &mut DelegateCtx,
        _: Target,
        cmd: &Command,
        view: &mut View,
        _: &Env,
    ) -> Handled {
        if let Some(new_view) = cmd.get(CHANGE_VIEW_CMD) {
            *view = new_view.clone();
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn get_window_location() -> (f64, f64) {
    let screen_size = Screen::get_monitors()
        .into_iter()
        .find(|m| m.is_primary())
        .map(|m| m.virtual_work_rect().size())
        .unwrap_or_default();

    (
        (screen_size.width - WINDOW_WIDTH) / 2.0,
        (screen_size.height - WINDOW_HEIGHT) / 2.0,
    )
}

fn make_window() -> StrResult {
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

        #[cfg(target_os = "linux")]
        gtk::init().map_err(err!())?;

        let window = WindowDesc::new(gui)
            .title("ALVR Launcher")
            .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_min_size((WINDOW_WIDTH, WINDOW_HEIGHT))
            .resizable(false)
            .set_position(get_window_location());

        let state = View::RequirementsCheck { steamvr: "".into() };

        let window_id = window.id;

        let app = AppLauncher::with_window(window)
            .use_simple_logger()
            .configure_env(|env, _| {
                env.set(theme::UI_FONT, FontDescriptor::default().with_size(15.0));
                env.set(theme::LABEL_COLOR, Color::rgb8(0, 0, 0));
                env.set(
                    theme::WINDOW_BACKGROUND_COLOR,
                    Color::rgb8(0xFF, 0xFF, 0xFF),
                );
                env.set(theme::WIDGET_PADDING_HORIZONTAL, 35);
                env.set(theme::WIDGET_PADDING_VERTICAL, 15);

                // button gradient
                env.set(theme::BUTTON_LIGHT, Color::rgb8(0xF0, 0xF0, 0xF0));
                env.set(theme::BUTTON_DARK, Color::rgb8(0xCC, 0xCC, 0xCC));
            })
            .delegate(Delegate);

        let handle = app.get_external_handle();
        thread::spawn(move || launcher_lifecycle(handle, window_id));

        app.launch(state).map_err(err!())?;
    }
    Ok(())
}

fn main() {
    let listener = TcpListener::bind(ALVR_LAUNCHER_ADDRESS).unwrap();

    alvr_common::show_err(make_window());
    start_watcher_thread();
    let mut is_driver_restarting = false;
    let mut is_update_in_progress = false;
    for conn in listener.incoming() {
        let packet = match handle_driver_launcher_connection(conn) {
            Some(value) => value,
            None => continue,
        };

        match packet.message {
            LauncherMessages::Shutdown => {
                if is_driver_restarting || is_update_in_progress {
                    // don't exit if we are expecting restart from driver or updating
                    continue;
                }
                commands::kill_steamvr();
                process::exit(0);
            }
            LauncherMessages::DriverStarted => {
                is_driver_restarting = false;
            }
            LauncherMessages::RestartSteamvr => {
                is_driver_restarting = true;
                commands::restart_steamvr();
            }
            LauncherMessages::Update => {
                if cfg!(windows) {
                    commands::invoke_installer();
                    return;
                } else {
                    show_e("Auto-updating on non-windows OS is unsupported.");
                }
            }
        }
    }
}

fn start_watcher_thread() {
    thread::spawn(move || {
        let mut steamvr_not_running_counter = 0;
        loop {
            thread::sleep(Duration::from_secs(2));
            if steamvr_not_running_counter >= 5 {
                send_launcher_packet(LauncherPacket {
                    message: LauncherMessages::Shutdown,
                });
            }
            if !is_steamvr_running() {
                steamvr_not_running_counter += 1;
            } else {
                steamvr_not_running_counter = 0;
            }
        }
    });
}

fn handle_driver_launcher_connection(
    conn: Result<std::net::TcpStream, io::Error>,
) -> Option<LauncherPacket> {
    let conn = conn.unwrap();
    let mut buffer = BufReader::new(conn);
    let mut data = Vec::new();
    let bytes_read = buffer.read_to_end(&mut data);
    match bytes_read {
        Ok(bytes_read) => {
            if bytes_read == 0 {
                show_e("Empty launcher packet received");
                return None;
            }
            let packet: LauncherPacket = serde_json::from_slice(&data).unwrap();
            Some(packet)
        }
        Err(e) => {
            show_e("Error occurred while receiving launcher message, is another program trying to communicate with ALVR?");
            show_e(e);
            None
        }
    }
}
