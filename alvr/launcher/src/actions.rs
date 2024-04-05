use crate::{
    InstallationInfo, Progress, ReleaseChannelsInfo, ReleaseInfo, UiMessage, WorkerMessage,
};
use alvr_common::{anyhow::Result, ToAny};
use anyhow::bail;
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

#[cfg(not(windows))]
const ADB_EXECUTABLE: &str = "adb";
#[cfg(windows)]
const ADB_EXECUTABLE: &str = "adb.exe";

#[cfg(target_os = "linux")]
const PLATFORM_TOOLS_DL_LINK: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-linux.zip";
#[cfg(target_os = "macos")]
const PLATFORM_TOOLS_DL_LINK: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-macos.zip";
#[cfg(windows)]
const PLATFORM_TOOLS_DL_LINK: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-windows.zip";

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
                        install_apk(&worker_message_sender, release_info, &client).await
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
            version: value["tag_name"].as_str().to_any()?.to_string(),
            assets: value["assets"]
                .as_array()
                .to_any()?
                .iter()
                .filter_map(|value| {
                    Some((
                        value["name"].as_str()?.to_string(),
                        value["browser_download_url"].as_str()?.to_string(),
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

async fn install_apk(
    worker_message_sender: &Sender<WorkerMessage>,
    release: ReleaseInfo,
    client: &reqwest::Client,
) -> anyhow::Result<()> {
    worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
        message: "Starting install".to_string(),
        progress: 0.0,
    }))?;

    let installation_dir = installations_dir().join(&release.version);

    let apk_path = installation_dir.clone().join(APK_NAME);

    if !apk_path.exists() {
        let apk_buffer = download(
            worker_message_sender,
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

    worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
        message: "Installing APK".to_string(),
        progress: 0.0,
    }))?;

    let res = match Command::new(ADB_EXECUTABLE)
        .arg("install")
        .arg("-d")
        .arg(&apk_path)
        .output()
    {
        Ok(res) => res,
        Err(_) => {
            let adb_path = data_dir().join("platform-tools").join(ADB_EXECUTABLE);

            if !adb_path.exists() {
                let mut buffer = Cursor::new(
                    download(
                        worker_message_sender,
                        "Downloading Android Platform Tools",
                        PLATFORM_TOOLS_DL_LINK,
                        client,
                    )
                    .await?,
                );

                zip::ZipArchive::new(&mut buffer)?.extract(&data_dir())?;
            }

            worker_message_sender.send(WorkerMessage::ProgressUpdate(Progress {
                message: "Installing APK".to_string(),
                progress: 0.0,
            }))?;

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
                    message: message.to_string(),
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
        message: "Starting install".to_string(),
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
        env::current_dir()
            .expect("Unable to determine executable directory")
            .join("ALVR-Launcher")
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
                            version: entry.file_name().to_string_lossy().to_string(),
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
