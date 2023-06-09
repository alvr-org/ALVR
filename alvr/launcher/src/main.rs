#[cfg(target_os = "linux")]
use std::os::unix::prelude::PermissionsExt;
use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs::{self, File, Permissions},
    io::{Cursor, Write},
    path::PathBuf,
    process::Command,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use eframe::{
    egui::{
        Button, CentralPanel, ComboBox, Context, Frame, Layout, ProgressBar, Style, TextEdit,
        Window,
    },
    emath::{Align, Align2},
    wgpu::Buffer,
};
use futures_util::StreamExt;

#[cfg(target_os = "linux")]
const DASHBOARD_PATHS: &[&str] = &["ALVR-x86_64.AppImage", "bin/alvr_dashboard"];
#[cfg(target_os = "windows")]
const DASHBOARD_PATHS: &[&str] = &["ALVR Dashboard.exe"];

#[derive(PartialEq, Eq, Copy, Clone)]
enum InstallationType {
    #[cfg(target_os = "linux")]
    AppImage,
    Archive,
}

impl ToString for InstallationType {
    fn to_string(&self) -> String {
        match self {
            #[cfg(target_os = "linux")]
            Self::AppImage => "AppImage",
            Self::Archive => "Archive",
        }
        .to_string()
    }
}

impl Default for InstallationType {
    #[allow(unreachable_code)]
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        return Self::AppImage;

        Self::Archive
    }
}

#[derive(Clone)]
struct Release {
    tag: String,
    assets: BTreeMap<String, String>,
}

impl Release {
    async fn fetch_releases(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<Self>> {
        let response: serde_json::Value = client.get(url).send().await?.json().await?;

        let mut releases = Vec::new();
        for value in response.as_array().unwrap() {
            releases.push(Self {
                tag: value["tag_name"].as_str().unwrap().to_string(),
                assets: value["assets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| {
                        (
                            value["name"].as_str().unwrap().to_string(),
                            value["browser_download_url"].as_str().unwrap().to_string(),
                        )
                    })
                    .collect(),
            })
        }
        Ok(releases)
    }
}

struct ReleaseData {
    stable: Vec<Release>,
    nightly: Vec<Release>,
}

impl ReleaseData {
    async fn fetch(client: &reqwest::Client) -> anyhow::Result<Self> {
        Ok(Self {
            stable: Release::fetch_releases(
                client,
                "https://api.github.com/repos/alvr-org/ALVR/releases",
            )
            .await?,
            nightly: Release::fetch_releases(
                client,
                "https://api.github.com/repos/alvr-org/ALVR-nightly/releases",
            )
            .await?,
        })
    }
}

enum State {
    Default,
    Installing(Progress),
}

enum WorkerMsg {
    VersionData(ReleaseData),
    ProgressUpdate(Progress),
    InstallDone,
    InstallFailed(String),
}

struct Progress {
    msg: String,
    progress: f32,
}

enum GuiMsg {
    Install {
        installation_type: InstallationType,
        release: Release,
    },
    Quit,
}

enum Popup {
    None,
    Edit(EditPopup),
    Version(VersionPopup),
}

struct EditPopup {
    version: String,
}

#[derive(Clone, PartialEq, Eq)]
enum Version {
    Stable(String),
    Nightly(String),
}

impl Version {
    fn inner(&self) -> &String {
        match self {
            Version::Stable(version) => version,
            Version::Nightly(version) => version,
        }
    }
}

impl ToString for Version {
    fn to_string(&self) -> String {
        match self {
            Version::Stable(version) => {
                format!("Stable {}", version)
            }
            Version::Nightly(version) => {
                format!("Nightly {}", version)
            }
        }
    }
}

struct VersionPopup {
    version: Version,
    installation_type: InstallationType,
}

struct Launcher {
    rx: Receiver<WorkerMsg>,
    tx: Sender<GuiMsg>,
    state: State,
    installations: Vec<String>,
    version_data: Option<ReleaseData>,
    popup: Popup,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext, rx: Receiver<WorkerMsg>, tx: Sender<GuiMsg>) -> Self {
        Self {
            rx,
            tx,
            state: State::Default,
            installations: get_installations(),
            version_data: None,
            popup: Popup::None,
        }
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMsg::VersionData(data) => self.version_data = Some(data),
                WorkerMsg::ProgressUpdate(progress) => {
                    self.state = State::Installing(progress);
                }
                WorkerMsg::InstallDone => {
                    // Refresh installations
                    self.installations = get_installations();
                    self.state = State::Default;
                }
                WorkerMsg::InstallFailed(why) => todo!(),
            }
        }

        CentralPanel::default().show(ctx, |ui| match &self.state {
            State::Default => {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label("ALVR Launcher");
                    ui.label(match &self.version_data {
                        Some(data) => format!("Latest stable release: {}", data.stable[0].tag),
                        None => "Fetching latest release...".to_string(),
                    });

                    for installation in &self.installations {
                        Frame::group(&Style::default()).show(ui, |ui| {
                            ui.columns(2, |ui| {
                                ui[0].label(installation);
                                ui[1].with_layout(Layout::right_to_left(Align::Min), |ui| {
                                    if ui.button("Launch").clicked() {
                                        for exec in DASHBOARD_PATHS {
                                            let mut path = data_dir();
                                            path.push(installation);
                                            path.push(exec);

                                            if Command::new(path).spawn().is_ok() {
                                                break;
                                            }
                                        }
                                        self.tx.send(GuiMsg::Quit).unwrap();
                                        frame.close();
                                    }
                                    if ui.button("Edit").clicked() {
                                        self.popup = Popup::Edit(EditPopup {
                                            version: installation.clone(),
                                        });
                                    }
                                    ui.button("Install APK");
                                })
                            })
                        });
                    }

                    if ui
                        .add_enabled(self.version_data.is_some(), Button::new("Add version"))
                        .clicked()
                    {
                        self.popup = Popup::Version(VersionPopup {
                            version: Version::Stable(
                                self.version_data.as_ref().unwrap().stable[0].tag.clone(),
                            ),
                            installation_type: InstallationType::default(),
                        });
                    }

                    let close_popup = match &mut self.popup {
                        Popup::Version(version_popup) => Window::new("Add version")
                            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
                            .resizable(false)
                            .collapsible(false)
                            .show(ctx, |ui| {
                                let version_data = self.version_data.as_ref().unwrap();
                                ui.columns(2, |ui| {
                                    ui[0].label("Version");

                                    let (channel, version_str, versions): (
                                        &str,
                                        String,
                                        Vec<Version>,
                                    ) = match version_popup.version.clone() {
                                        Version::Stable(version) => (
                                            "Stable",
                                            version,
                                            version_data
                                                .stable
                                                .iter()
                                                .map(|release| Version::Stable(release.tag.clone()))
                                                .collect(),
                                        ),
                                        Version::Nightly(version) => (
                                            "Nightly",
                                            version,
                                            version_data
                                                .nightly
                                                .iter()
                                                .map(|release| {
                                                    Version::Nightly(release.tag.clone())
                                                })
                                                .collect(),
                                        ),
                                    };

                                    ComboBox::from_label("Channel")
                                        .selected_text(channel)
                                        .show_ui(&mut ui[1], |ui| {
                                            ui.selectable_value(
                                                &mut version_popup.version,
                                                Version::Stable(
                                                    self.version_data.as_ref().unwrap().stable[0]
                                                        .tag
                                                        .clone(),
                                                ),
                                                "Stable",
                                            );
                                            ui.selectable_value(
                                                &mut version_popup.version,
                                                Version::Nightly(
                                                    self.version_data.as_ref().unwrap().nightly[0]
                                                        .tag
                                                        .clone(),
                                                ),
                                                "Nightly",
                                            );
                                        });
                                    ComboBox::from_label("Version")
                                        .selected_text(version_str)
                                        .show_ui(&mut ui[1], |ui| {
                                            for version in versions {
                                                ui.selectable_value(
                                                    &mut version_popup.version,
                                                    version.clone(),
                                                    version.inner(),
                                                );
                                            }
                                        });
                                });
                                ui.columns(2, |ui| {
                                    ui[0].label("Installation");
                                    ComboBox::from_label("Type")
                                        .selected_text(version_popup.installation_type.to_string())
                                        .show_ui(&mut ui[1], |ui| {
                                            #[cfg(target_os = "linux")]
                                            ui.selectable_value(
                                                &mut version_popup.installation_type,
                                                InstallationType::AppImage,
                                                InstallationType::AppImage.to_string(),
                                            );
                                            ui.selectable_value(
                                                &mut version_popup.installation_type,
                                                InstallationType::Archive,
                                                InstallationType::Archive.to_string(),
                                            );
                                        });
                                });

                                ui.columns(2, |ui| {
                                    if ui[0].button("Install").clicked() {
                                        self.tx
                                            .send(GuiMsg::Install {
                                                installation_type: version_popup.installation_type,
                                                release: match &version_popup.version {
                                                    Version::Stable(version) => version_data
                                                        .stable
                                                        .iter()
                                                        .find(|release| release.tag == *version)
                                                        .unwrap()
                                                        .clone(),
                                                    Version::Nightly(version) => version_data
                                                        .nightly
                                                        .iter()
                                                        .find(|release| release.tag == *version)
                                                        .unwrap()
                                                        .clone(),
                                                },
                                            })
                                            .unwrap();
                                        return true;
                                    }

                                    // Close window later if Cancel is clicked
                                    ui[1].button("Cancel").clicked()
                                })
                            })
                            .unwrap()
                            .inner
                            .unwrap(),
                        Popup::Edit(EditPopup { version }) => Window::new("Edit version")
                            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
                            .resizable(false)
                            .collapsible(false)
                            .show(ctx, |ui| {
                                if ui.button("Delete version").clicked() {
                                    let mut path = data_dir();
                                    path.push(version);
                                    fs::remove_dir_all(path).expect("Failed to delete version");

                                    self.installations = get_installations();

                                    return true;
                                };
                                ui.button("Close").clicked()
                            })
                            .unwrap()
                            .inner
                            .unwrap(),
                        _ => false,
                    };
                    if close_popup {
                        self.popup = Popup::None;
                    }
                });
            }
            State::Installing(progress) => {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(&progress.msg);
                    ui.add(ProgressBar::new(progress.progress).animate(true));
                });
            }
        });
    }

    fn on_close_event(&mut self) -> bool {
        self.tx.send(GuiMsg::Quit).unwrap();
        true
    }
}

fn main() {
    let (worker_tx, gui_rx) = mpsc::channel::<WorkerMsg>();
    let (gui_tx, worker_rx) = mpsc::channel::<GuiMsg>();

    let worker_handle = thread::spawn(|| worker(worker_rx, worker_tx));

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

fn worker(rx: Receiver<GuiMsg>, tx: Sender<WorkerMsg>) {
    tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            let client = reqwest::Client::builder()
                .user_agent("ALVR-Launcher")
                .build()
                .unwrap();
            let version_data = match ReleaseData::fetch(&client).await {
                Ok(data) => data,
                Err(why) => {
                    eprintln!("Error fetching version data: {}", why);
                    return;
                }
            };

            tx.send(WorkerMsg::VersionData(version_data)).unwrap();

            loop {
                match rx.recv().unwrap() {
                    GuiMsg::Quit => return,
                    GuiMsg::Install {
                        installation_type,
                        release,
                    } => {
                        tx.send(WorkerMsg::ProgressUpdate(Progress {
                            msg: "Starting install".to_string(),
                            progress: 0.0,
                        }))
                        .unwrap();

                        let url = match installation_type {
                            #[cfg(target_os = "linux")]
                            InstallationType::AppImage => {
                                release.assets["ALVR-x86_64.AppImage"].clone()
                            }
                            #[cfg(target_os = "linux")]
                            InstallationType::Archive => {
                                release.assets["alvr_streamer_linux.tar.gz"].clone()
                            }
                            #[cfg(target_os = "windows")]
                            InstallationType::Archive => {
                                release.assets["alvr_streamer_windows.zip"].clone()
                            }
                        };

                        let res = client.get(url).send().await.unwrap();

                        let total_size = res.content_length().unwrap();

                        let mut stream = res.bytes_stream();
                        let mut buffer = Vec::new();

                        while let Some(item) = stream.next().await {
                            buffer.extend(item.unwrap());

                            tx.send(WorkerMsg::ProgressUpdate(Progress {
                                msg: "Downloading".to_string(),
                                progress: buffer.len() as f32 / total_size as f32,
                            }))
                            .unwrap();
                        }

                        let mut installation_dir = data_dir();
                        installation_dir.push(&release.tag);

                        fs::create_dir_all(&installation_dir).unwrap();

                        match installation_type {
                            #[cfg(target_os = "linux")]
                            InstallationType::AppImage => {
                                installation_dir.push("ALVR-x86_64.AppImage");
                                let mut file = File::create(&installation_dir).unwrap();

                                file.write_all(&buffer).unwrap();
                                file.set_permissions(Permissions::from_mode(0o755)).unwrap();
                            }
                            #[cfg(target_os = "linux")]
                            InstallationType::Archive => todo!(),
                            #[cfg(target_os = "windows")]
                            InstallationType::Archive => {
                                let buffer = Cursor::new(buffer);
                                zip::ZipArchive::new(&buffer)
                                    .extract(&installation_dir)
                                    .expect("Failed to extract streamer");
                            }
                        }

                        tx.send(WorkerMsg::InstallDone).unwrap();
                    }
                }
            }
        });
}

fn data_dir() -> PathBuf {
    if cfg!(target_os = "linux") {
        let mut path = PathBuf::from(env::var("HOME").expect("Failed to determine home directory"));
        path.push(".local/share/ALVR-Launcher");
        path
    } else if cfg!(target_os = "windows") {
        let mut path = env::current_dir().expect("Unable to determine executable directory");
        path.push("ALVR-Launcher");
        path
    } else {
        panic!("Unsupported OS")
    }
}

fn get_installations() -> Vec<String> {
    match fs::read_dir(data_dir()) {
        Ok(entries) => entries
            .into_iter()
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|entry| entry.file_name().to_string_lossy().to_string())
            })
            .collect(),
        Err(why) => {
            eprintln!("Failed to read data dir: {}", why);
            Vec::new()
        }
    }
}
