#[cfg(target_os = "linux")]
use std::os::unix::prelude::PermissionsExt;
use std::{
    collections::BTreeMap,
    env,
    fs::{self, File, Permissions},
    io::{Cursor, Write},
    mem,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use eframe::{
    egui::{
        Button, CentralPanel, ComboBox, Context, Frame, Grid, Layout, ProgressBar, ViewportCommand,
        Window,
    },
    emath::{Align, Align2},
    epaint::Color32,
};
use futures_util::StreamExt;

#[cfg(not(target_os = "windows"))]
const DASHBOARD_PATHS: &[&str] = &["ALVR-x86_64.AppImage", "bin/alvr_dashboard"];
#[cfg(target_os = "windows")]
const DASHBOARD_PATHS: &[&str] = &["ALVR Dashboard.exe"];

#[cfg(not(target_os = "windows"))]
const ADB_EXECUTABLE: &str = "adb";
#[cfg(target_os = "windows")]
const ADB_EXECUTABLE: &str = "adb.exe";

#[cfg(not(target_os = "windows"))]
const PLATFORM_TOOLS_DL_LINK: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-linux.zip";
#[cfg(target_os = "windows")]
const PLATFORM_TOOLS_DL_LINK: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-windows.zip";

const VERSIONS_SUBDIR: &str = "versions";

const APK_NAME: &str = "client.apk";

trait Extended<P> {
    fn extended(self, path: P) -> Self;
}

impl<P> Extended<P> for PathBuf
where
    P: AsRef<Path>,
{
    fn extended(mut self, path: P) -> Self {
        self.push(path);
        self
    }
}

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

    fn get_release(&self, version: &str) -> Option<&Release> {
        self.stable
            .iter()
            .find(|release| release.tag == version)
            .or_else(|| self.nightly.iter().find(|release| release.tag == version))
    }
}

enum State {
    Default,
    Installing(Progress),
    Error(String),
}

enum WorkerMsg {
    VersionData(ReleaseData),
    ProgressUpdate(Progress),
    Done,
    Error(String),
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
    InstallClient(Release),
    Quit,
}

enum Popup {
    None,
    Delete(String),
    Edit(String),
    Version(VersionPopup),
}

impl Default for Popup {
    fn default() -> Self {
        Self::None
    }
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
    installations: Vec<Installation>,
    version_data: Option<ReleaseData>,
    popup: Popup,
}

struct Installation {
    version: String,
    apk_downloaded: bool,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext, rx: Receiver<WorkerMsg>, tx: Sender<GuiMsg>) -> Self {
        alvr_gui_common::theme::set_theme(&cc.egui_ctx);

        Self {
            rx,
            tx,
            state: State::Default,
            installations: get_installations(),
            version_data: None,
            popup: Popup::None,
        }
    }

    fn version_popup(&mut self, ctx: &Context, mut version_popup: VersionPopup) -> Popup {
        Window::new("Add version")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                let version_data = self.version_data.as_ref().unwrap();
                let (channel, version_str, versions): (&str, String, Vec<Version>) =
                    match version_popup.version.clone() {
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
                                .map(|release| Version::Nightly(release.tag.clone()))
                                .collect(),
                        ),
                    };
                Grid::new("add-version-grid").num_columns(2).show(ui, |ui| {
                    ui.label("Channel");

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ComboBox::from_id_source("channel")
                            .selected_text(channel)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut version_popup.version,
                                    Version::Stable(
                                        self.version_data.as_ref().unwrap().stable[0].tag.clone(),
                                    ),
                                    "Stable",
                                );
                                ui.selectable_value(
                                    &mut version_popup.version,
                                    Version::Nightly(
                                        self.version_data.as_ref().unwrap().nightly[0].tag.clone(),
                                    ),
                                    "Nightly",
                                );
                            })
                    });
                    ui.end_row();

                    ui.label("Version");
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ComboBox::from_id_source("version")
                            .selected_text(version_str)
                            .show_ui(ui, |ui| {
                                for version in versions {
                                    ui.selectable_value(
                                        &mut version_popup.version,
                                        version.clone(),
                                        version.inner(),
                                    );
                                }
                            })
                    });
                    ui.end_row();

                    ui.label("Installation Type");
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ComboBox::from_id_source("type")
                            .selected_text(version_popup.installation_type.to_string())
                            .show_ui(ui, |ui| {
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
                            })
                    });
                    ui.end_row();
                });
                ui.columns(2, |ui| {
                    if ui[0].button("Cancel").clicked() {
                        return Popup::None;
                    }

                    if ui[1].button("Install").clicked() {
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
                        return Popup::None;
                    }

                    Popup::Version(version_popup)
                })
            })
            .unwrap()
            .inner
            .unwrap()
    }

    fn edit_popup(&self, ctx: &Context, version: String) -> Popup {
        Window::new("Edit version")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                    if ui.button("Delete version").clicked() {
                        return Popup::Delete(version);
                    };
                    if ui.button("Close").clicked() {
                        return Popup::None;
                    }

                    Popup::Edit(version)
                })
                .inner
            })
            .unwrap()
            .inner
            .unwrap()
    }

    fn delete_popup(&mut self, ctx: &Context, version: String) -> Popup {
        Window::new("Are you sure?")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(format!("This will permanently delete version {}", version));
                });
                ui.columns(2, |ui| {
                    if ui[0].button("Cancel").clicked() {
                        return Popup::None;
                    }
                    if ui[1].button("Delete version").clicked() {
                        let mut path = data_dir();
                        path.push(version);
                        fs::remove_dir_all(path).expect("Failed to delete version");

                        self.installations = get_installations();

                        return Popup::None;
                    }
                    Popup::Delete(version)
                })
            })
            .unwrap()
            .inner
            .unwrap()
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMsg::VersionData(data) => self.version_data = Some(data),
                WorkerMsg::ProgressUpdate(progress) => {
                    self.state = State::Installing(progress);
                }
                WorkerMsg::Done => {
                    // Refresh installations
                    self.installations = get_installations();
                    self.state = State::Default;
                }
                WorkerMsg::Error(why) => self.state = State::Error(why),
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
                        let path = data_dir()
                            .extended(VERSIONS_SUBDIR)
                            .extended(&installation.version);

                        Frame::group(ui.style())
                            .fill(alvr_gui_common::theme::SECTION_BG)
                            .show(ui, |ui| {
                                Grid::new(&installation.version)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label(&installation.version);
                                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                            if ui.button("Launch").clicked() {
                                                for exec in DASHBOARD_PATHS {
                                                    let path = path.clone().extended(exec);

                                                    if Command::new(&path).spawn().is_ok() {
                                                        break;
                                                    }
                                                }
                                                self.tx.send(GuiMsg::Quit).unwrap();
                                                ctx.send_viewport_cmd(ViewportCommand::Close);
                                            }
                                            if ui
                                                .add_enabled(
                                                    self.version_data.is_some()
                                                        && self
                                                            .version_data
                                                            .as_ref()
                                                            .unwrap()
                                                            .get_release(&installation.version)
                                                            .is_some()
                                                        || installation.apk_downloaded,
                                                    Button::new("Install APK"),
                                                )
                                                .clicked()
                                            {
                                                self.tx
                                                    .send(GuiMsg::InstallClient(
                                                        self.version_data
                                                            .as_ref()
                                                            .unwrap()
                                                            .get_release(&installation.version)
                                                            .unwrap()
                                                            .clone(),
                                                    ))
                                                    .unwrap();
                                            };
                                            if ui.button("Open directory").clicked() {
                                                open::that_in_background(path);
                                            }
                                            if ui.button("Edit").clicked() {
                                                self.popup =
                                                    Popup::Edit(installation.version.clone());
                                            }
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

                    let popup = match mem::take(&mut self.popup) {
                        Popup::Version(version_popup) => self.version_popup(ctx, version_popup),
                        Popup::Edit(version) => self.edit_popup(ctx, version),
                        Popup::Delete(version) => self.delete_popup(ctx, version),
                        Popup::None => Popup::None,
                    };
                    self.popup = popup;
                });
            }
            State::Installing(progress) => {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(&progress.msg);
                    ui.add(ProgressBar::new(progress.progress).animate(true));
                });
            }
            State::Error(why) => {
                let why = why.clone(); // Avoid borrowing issues with the closure for the layout
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.colored_label(Color32::LIGHT_RED, "Error!");
                    ui.label(why);

                    if ui.button("Close").clicked() {
                        self.state = State::Default;
                    }
                });
            }
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            self.tx.send(GuiMsg::Quit).unwrap();
        }
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
                    } => match install(&tx, installation_type, release, &client).await {
                        Ok(_) => tx.send(WorkerMsg::Done).unwrap(),
                        Err(why) => tx.send(WorkerMsg::Error(why.to_string())).unwrap(),
                    },
                    GuiMsg::InstallClient(release) => {
                        match install_apk(&tx, release, &client).await {
                            Ok(_) => tx.send(WorkerMsg::Done).unwrap(),
                            Err(why) => tx.send(WorkerMsg::Error(why.to_string())).unwrap(),
                        }
                    }
                }
            }
        });
}

async fn install_apk(
    tx: &Sender<WorkerMsg>,
    release: Release,
    client: &reqwest::Client,
) -> anyhow::Result<()> {
    tx.send(WorkerMsg::ProgressUpdate(Progress {
        msg: "Starting install".to_string(),
        progress: 0.0,
    }))
    .unwrap();

    let installation_dir = data_dir().extended(VERSIONS_SUBDIR).extended(&release.tag);

    let apk_path = installation_dir.clone().extended(APK_NAME);

    if !apk_path.exists() {
        let apk_buffer = download(
            tx,
            "Downloading Client APK",
            release
                .assets
                .get("alvr_client_android.apk")
                .ok_or(anyhow::anyhow!("Unable to determine download URL"))?,
            client,
        )
        .await?;

        let mut file = File::create(&apk_path)?;
        file.write_all(&apk_buffer)?;
    }

    tx.send(WorkerMsg::ProgressUpdate(Progress {
        msg: "Installing APK".to_string(),
        progress: 0.0,
    }))
    .unwrap();

    let res = match Command::new(ADB_EXECUTABLE)
        .arg("install")
        .arg("-r")
        .arg(&apk_path)
        .output()
    {
        Ok(res) => res,
        Err(_) => {
            let adb_path = data_dir()
                .extended("platform-tools")
                .extended(ADB_EXECUTABLE);

            if !adb_path.exists() {
                let mut buffer = Cursor::new(
                    download(
                        tx,
                        "Downloading Android Platform Tools",
                        PLATFORM_TOOLS_DL_LINK,
                        client,
                    )
                    .await?,
                );

                zip::ZipArchive::new(&mut buffer)?.extract(&data_dir())?;
            }

            tx.send(WorkerMsg::ProgressUpdate(Progress {
                msg: "Installing APK".to_string(),
                progress: 0.0,
            }))
            .unwrap();

            Command::new(adb_path)
                .arg("install")
                .arg("-r")
                .arg(&apk_path)
                .output()?
        }
    };
    if res.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "ADB install failed: {}",
            String::from_utf8_lossy(&res.stderr)
        ))
    }
}

async fn download(
    tx: &Sender<WorkerMsg>,
    msg: &str,
    url: &str,
    client: &reqwest::Client,
) -> anyhow::Result<Vec<u8>> {
    let res = client.get(url).send().await?;
    let total_size = res.content_length();
    let mut stream = res.bytes_stream();
    let mut buffer = Vec::new();
    while let Some(item) = stream.next().await {
        buffer.extend(item?);

        match total_size {
            Some(total_size) => tx
                .send(WorkerMsg::ProgressUpdate(Progress {
                    msg: msg.to_string(),
                    progress: buffer.len() as f32 / total_size as f32,
                }))
                .unwrap(),
            None => tx
                .send(WorkerMsg::ProgressUpdate(Progress {
                    msg: format!("{} (Progress unavailable)", msg),
                    progress: 0.5,
                }))
                .unwrap(),
        }
    }

    Ok(buffer)
}

async fn install(
    tx: &Sender<WorkerMsg>,
    installation_type: InstallationType,
    release: Release,
    client: &reqwest::Client,
) -> anyhow::Result<()> {
    tx.send(WorkerMsg::ProgressUpdate(Progress {
        msg: "Starting install".to_string(),
        progress: 0.0,
    }))
    .unwrap();

    let url = match installation_type {
        #[cfg(target_os = "linux")]
        InstallationType::AppImage => release.assets.get("ALVR-x86_64.AppImage"),
        #[cfg(not(target_os = "windows"))]
        InstallationType::Archive => release.assets.get("alvr_streamer_linux.tar.gz"),
        #[cfg(target_os = "windows")]
        InstallationType::Archive => release.assets.get("alvr_streamer_windows.zip"),
    }
    .ok_or(anyhow::anyhow!("Unable to determine download link"))?
    .clone();

    let buffer = download(tx, "Downloading Streamer", &url, client).await?;

    let mut installation_dir = data_dir().extended(VERSIONS_SUBDIR).extended(&release.tag);

    fs::create_dir_all(&installation_dir)?;

    match installation_type {
        #[cfg(target_os = "linux")]
        InstallationType::AppImage => {
            installation_dir.push("ALVR-x86_64.AppImage");
            let mut file = File::create(&installation_dir)?;

            file.write_all(&buffer)?;
            file.set_permissions(Permissions::from_mode(0o755))?;
        }
        #[cfg(not(target_os = "windows"))]
        InstallationType::Archive => todo!(),
        #[cfg(target_os = "windows")]
        InstallationType::Archive => {
            let mut buffer = Cursor::new(buffer);
            zip::ZipArchive::new(&mut buffer)?.extract(&installation_dir)?;
        }
    }
    Ok(())
}

fn data_dir() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from(env::var("HOME").expect("Failed to determine home directory"))
            .extended(".local/share/ALVR-Launcher")
    } else {
        env::current_dir()
            .expect("Unable to determine executable directory")
            .extended("ALVR-Launcher")
    }
}

fn get_installations() -> Vec<Installation> {
    match fs::read_dir(data_dir().extended(VERSIONS_SUBDIR)) {
        Ok(entries) => entries
            .into_iter()
            .filter_map(|entry| {
                entry
                    .ok()
                    .filter(|entry| match entry.file_type() {
                        Ok(file_type) => file_type.is_dir(),
                        Err(why) => {
                            eprintln!("Failed to read entry file type: {}", why);
                            false
                        }
                    })
                    .map(|entry| {
                        let mut apk_path = entry.path();
                        apk_path.push(APK_NAME);
                        Installation {
                            version: entry.file_name().to_string_lossy().to_string(),
                            apk_downloaded: apk_path.exists(),
                        }
                    })
            })
            .collect(),
        Err(why) => {
            eprintln!("Failed to read versions dir: {}", why);
            Vec::new()
        }
    }
}
