use crate::{
    InstallationInfo, Progress, ReleaseChannelsInfo, ReleaseInfo, UiMessage, WorkerMessage,
};
use alvr_common::{anyhow::Result, semver::Version, ToAny};
use anyhow::{bail, Context};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use std::{
    env,
    fs::{self, File},
    io::{Cursor, Write},
    path::PathBuf,
    process::Command,
    sync::mpsc::{Receiver, Sender},
};

const APK_NAME: &str = "client.apk";

pub fn installations_dir() -> PathBuf {
    data_dir().join("installations")
}

pub fn worker(
    ui_message_receiver: Receiver<UiMessage>,
    worker_message_sender: Sender<WorkerMessage>,
) {
    tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            let client = reqwest::Client::builder()
                .user_agent("ALVR-Launcher")
                .build()
                .unwrap();
            let version_data = match fetch_all_releases(&client).await {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error fetching version data: {}", e);
                    return;
                }
            };

            worker_message_sender
                .send(WorkerMessage::ReleaseChannelsInfo(version_data))
                .unwrap();

            loop {
                let Ok(message) = ui_message_receiver.recv() else {
                    return;
                };
                let res = match message {
                    UiMessage::Quit => return,
                    UiMessage::InstallServer(release) => {
                        install_server(&worker_message_sender, release, &client).await
                    }
                    UiMessage::InstallClient(release_info) => {
                        install_and_launch_apk(&worker_message_sender, release_info)
                    }
                };
                match res {
                    Ok(()) => worker_message_sender.send(WorkerMessage::Done).unwrap(),
                    Err(e) => worker_message_sender
                        .send(WorkerMessage::Error(e.to_string()))
                        .unwrap(),
                }
            }
        });
}

async fn fetch_all_releases(client: &reqwest::Client) -> Result<ReleaseChannelsInfo> {
    Ok(ReleaseChannelsInfo {
        stable: fetch_releases_for_repo(
            client,
            "https://api.github.com/repos/alvr-org/ALVR/releases",
        )
        .await?,
        nightly: fetch_releases_for_repo(
            client,
            "https://api.github.com/repos/alvr-org/ALVR-nightly/releases",
        )
        .await?,
    })
}

async fn fetch_releases_for_repo(client: &reqwest::Client, url: &str) -> Result<Vec<ReleaseInfo>> {
    let response: serde_json::Value = client.get(url).send().await?.json().await?;

    let mut releases = Vec::new();
    for value in response.as_array().to_any()? {
        releases.push(ReleaseInfo {
            version: value["tag_name"].as_str().to_any()?.into(),
            assets: value["assets"]
                .as_array()
                .to_any()?
                .iter()
                .filter_map(|value| {
                    Some((
                        value["name"].as_str()?.into(),
                        value["browser_download_url"].as_str()?.into(),
                    ))
                })
                .collect(),
        })
    }
    Ok(releases)
}

pub fn get_release(
    release_channels_info: &ReleaseChannelsInfo,
    version: &str,
) -> Option<ReleaseInfo> {
    release_channels_info
        .stable
        .iter()
        .find(|release| release.version == version)
        .cloned()
        .or_else(|| {
            release_channels_info
                .nightly
                .iter()
                .find(|release| release.version == version)
                .cloned()
        })
}

fn install_and_launch_apk(
    worker_message_sender: &Sender<WorkerMessage>,
    release: ReleaseInfo,
) -> anyhow::Result<()> {
    worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
        message: "Starting install".into(),
        progress: 0.0,
    }))?;

    let root = installations_dir().join(&release.version);
    let apk_name = "alvr_client_android.apk";
    let apk_path = root.join(apk_name);
    if !apk_path.exists() {
        let apk_url = release
            .assets
            .get(apk_name)
            .ok_or(anyhow::anyhow!("Unable to determine download URL"))?;
        let apk_buffer = alvr_adb::commands::download(apk_url, |downloaded, total| {
            let progress = total.map(|t| downloaded as f32 / t as f32).unwrap_or(0.0);
            worker_message_sender
                .send(WorkerMessage::ProgressUpdate(Progress {
                    message: "Downloading Client APK".into(),
                    progress,
                }))
                .ok();
        })?;
        let mut file = File::create(&apk_path)?;
        file.write_all(&apk_buffer)?;
    }

    let layout = alvr_filesystem::Layout::new(&root);
    let adb_path = alvr_adb::commands::require_adb(&layout, |downloaded, total| {
        let progress = total.map(|t| downloaded as f32 / t as f32).unwrap_or(0.0);
        worker_message_sender
            .send(WorkerMessage::ProgressUpdate(Progress {
                message: "Downloading ADB".into(),
                progress,
            }))
            .ok();
    })?;

    let device_serial = alvr_adb::commands::list_devices(&adb_path)?
        .iter()
        .find_map(|d| d.serial.clone())
        .ok_or(anyhow::anyhow!("Failed to find connected device"))?;

    let v = if release.version.starts_with('v') {
        release.version[1..].to_string()
    } else {
        release.version
    };
    let version = Version::parse(&v).context("Failed to parse release version")?;
    let stable = version.pre.is_empty() && !version.build.contains("nightly");
    let application_id = if stable {
        alvr_system_info::PACKAGE_NAME_GITHUB_STABLE
    } else {
        alvr_system_info::PACKAGE_NAME_GITHUB_DEV
    };

    if alvr_adb::commands::is_package_installed(&adb_path, &device_serial, application_id)? {
        worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
            message: "Uninstalling old APK".into(),
            progress: 0.0,
        }))?;
        alvr_adb::commands::uninstall_package(&adb_path, &device_serial, application_id)?;
    }

    worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
        message: "Installing new APK".into(),
        progress: 0.0,
    }))?;
    alvr_adb::commands::install_package(&adb_path, &device_serial, &apk_path.to_string_lossy())?;

    alvr_adb::commands::start_application(&adb_path, &device_serial, application_id)?;

    Ok(())
}

async fn download(
    worker_message_sender: &Sender<WorkerMessage>,
    message: &str,
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
            Some(total_size) => {
                worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
                    message: message.into(),
                    progress: buffer.len() as f32 / total_size as f32,
                }))?
            }
            None => worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
                message: format!("{} (Progress unavailable)", message),
                progress: 0.5,
            }))?,
        }
    }

    Ok(buffer)
}

async fn install_server(
    worker_message_sender: &Sender<WorkerMessage>,
    release: ReleaseInfo,
    client: &reqwest::Client,
) -> anyhow::Result<()> {
    worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
        message: "Starting install".into(),
        progress: 0.0,
    }))?;

    let file_name = if cfg!(windows) {
        "alvr_streamer_windows.zip"
    } else {
        "alvr_streamer_linux.tar.gz"
    };

    let url = release
        .assets
        .get(file_name)
        .ok_or(anyhow::anyhow!("Unable to determine download link"))?;

    let buffer = download(worker_message_sender, "Downloading Streamer", url, client).await?;

    let installation_dir = installations_dir().join(&release.version);

    fs::create_dir_all(&installation_dir)?;

    let mut buffer = Cursor::new(buffer);
    if cfg!(windows) {
        zip::ZipArchive::new(&mut buffer)?.extract(&installation_dir)?;
    } else {
        tar::Archive::new(&mut GzDecoder::new(&mut buffer)).unpack(&installation_dir)?;
    }

    Ok(())
}

pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from(env::var("HOME").expect("Failed to determine home directory"))
            .join(".local/share/ALVR-Launcher")
    } else {
        env::current_exe()
            .expect("Unable to determine executable directory")
            .parent()
            .unwrap()
            .to_owned()
    }
}

pub fn get_installations() -> Vec<InstallationInfo> {
    match fs::read_dir(installations_dir()) {
        Ok(entries) => entries
            .into_iter()
            .filter_map(|entry| {
                entry
                    .ok()
                    .filter(|entry| match entry.file_type() {
                        Ok(file_type) => file_type.is_dir(),
                        Err(e) => {
                            eprintln!("Failed to read entry file type: {}", e);
                            false
                        }
                    })
                    .map(|entry| {
                        let mut apk_path = entry.path();
                        apk_path.push(APK_NAME);
                        InstallationInfo {
                            version: entry.file_name().to_string_lossy().into(),
                            is_apk_downloaded: apk_path.exists(),
                        }
                    })
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed to read versions dir: {}", e);
            Vec::new()
        }
    }
}

pub fn launch_dashboard(version: &str) -> Result<()> {
    let installation_dir = installations_dir().join(version);

    let dashboard_path = if cfg!(windows) {
        installation_dir.join("ALVR Dashboard.exe")
    } else if cfg!(target_os = "linux") {
        installation_dir.join("alvr_streamer_linux/bin/alvr_dashboard")
    } else {
        bail!("Unsupported platform")
    };

    Command::new(dashboard_path).spawn()?;

    Ok(())
}

pub fn delete_installation(version: &str) -> Result<()> {
    fs::remove_dir_all(installations_dir().join(version))?;

    Ok(())
}
