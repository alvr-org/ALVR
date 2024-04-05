mod actions;
mod ui;

use eframe::egui::{IconData, ViewportBuilder};
use ico::IconDir;
use std::{collections::BTreeMap, io::Cursor, sync::mpsc, thread};
use ui::Launcher;

#[derive(Clone)]
struct ReleaseInfo {
    version: String,
    assets: BTreeMap<String, String>,
}

struct Progress {
    message: String,
    progress: f32,
}

enum WorkerMessage {
    ReleaseChannelsInfo(ReleaseChannelsInfo),
    ProgressUpdate(Progress),
    Done,
    Error(String),
}

enum UiMessage {
    InstallServer(ReleaseInfo),
    InstallClient(ReleaseInfo),
    Quit,
}

struct ReleaseChannelsInfo {
    stable: Vec<ReleaseInfo>,
    nightly: Vec<ReleaseInfo>,
}

struct InstallationInfo {
    pub version: String,
    is_apk_downloaded: bool,
}

fn main() {
    let (worker_message_sender, worker_message_receiver) = mpsc::channel::<WorkerMessage>();
    let (ui_message_sender, ui_message_receiver) = mpsc::channel::<UiMessage>();

    let worker_handle =
        thread::spawn(|| actions::worker(ui_message_receiver, worker_message_sender));

    let ico = IconDir::read(Cursor::new(include_bytes!(
        "../../dashboard/resources/dashboard.ico"
    )))
    .unwrap();
    let image = ico.entries().first().unwrap().decode().unwrap();

    eframe::run_native(
        "ALVR Launcher",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size((700.0, 400.0))
                .with_icon(IconData {
                    rgba: image.rgba_data().to_owned(),
                    width: image.width(),
                    height: image.height(),
                }),
            ..Default::default()
        },
        Box::new(move |cc| {
            Box::new(Launcher::new(
                cc,
                worker_message_receiver,
                ui_message_sender,
            ))
        }),
    )
    .expect("Failed to run eframe");

    worker_handle.join().unwrap();
}
