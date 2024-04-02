mod actions;
mod ui;

use std::{collections::BTreeMap, sync::mpsc, thread};
use ui::Launcher;

#[derive(Clone)]
struct ReleaseInfo {
    tag: String,
    assets: BTreeMap<String, String>,
}

struct Progress {
    msg: String,
    progress: f32,
}

enum WorkerMessage {
    ReleaseChannelsInfo(ReleaseChannelsInfo),
    ProgressUpdate(Progress),
    Done,
    Error(String),
}

enum UiMessage {
    Install(ReleaseInfo),
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
    let (worker_tx, gui_rx) = mpsc::channel::<WorkerMessage>();
    let (gui_tx, worker_rx) = mpsc::channel::<UiMessage>();

    let worker_handle = thread::spawn(|| actions::worker(worker_rx, worker_tx));

    eframe::run_native(
        "ALVR Launcher",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(move |cc| Box::new(Launcher::new(cc, gui_rx, gui_tx))),
    )
    .expect("Failed to run eframe");

    worker_handle.join().unwrap();
}
