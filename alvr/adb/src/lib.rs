// https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md

use std::{io::Cursor, path::PathBuf, process::Command};

use const_format::formatcp;
use futures_util::StreamExt;
use zip::ZipArchive;

#[cfg(not(windows))]
const ADB_EXECUTABLE: &str = "adb";
#[cfg(windows)]
const ADB_EXECUTABLE: &str = "adb.exe";

// https://developer.android.com/tools/releases/platform-tools#revisions
// NOTE: At the time of writing this comment, the revisions section above
// shows the latest version as 35.0.2, but the latest that can be downloaded
// by specifying a version is 35.0.0
const PLATFORM_TOOLS_VERSION: &str = "-latest"; // E.g. "_r35.0.0"

#[cfg(target_os = "linux")]
const PLATFORM_TOOLS_OS: &str = "linux";
#[cfg(target_os = "macos")]
const PLATFORM_TOOLS_OS: &str = "darwin";
#[cfg(windows)]
const PLATFORM_TOOLS_OS: &str = "windows";

const PLATFORM_TOOLS_URL: &str = formatcp!("https://dl.google.com/android/repository/platform-tools{PLATFORM_TOOLS_VERSION}-{PLATFORM_TOOLS_OS}.zip");

///////////////////
// ADB Installation

type ProgressCallback = fn(u64, usize);

/// Installs a local version of `adb` in the specified `path`, using `client` to download the "platform-tools" archive.
pub async fn install_adb(
    client: &reqwest::Client,
    progress_callback: ProgressCallback,
    path: PathBuf,
) -> anyhow::Result<()> {
    let buffer = download_adb(&client, progress_callback).await?;
    let mut reader = Cursor::new(buffer);
    ZipArchive::new(&mut reader)?.extract(path)?;
    Ok(())
}

async fn download_adb(
    client: &reqwest::Client,
    progress_callback: ProgressCallback,
) -> anyhow::Result<Vec<u8>> {
    let response = client.get(PLATFORM_TOOLS_URL).send().await?;
    let maybe_total_size = response.content_length();
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    while let Some(item) = stream.next().await {
        buffer.extend(item?);
        if let Some(total_size) = maybe_total_size {
            let downloaded_size = buffer.len();
            (progress_callback)(total_size, downloaded_size)
        }
    }
    Ok(buffer)
}

///////////////////
// APK installation

pub fn install_apk(adb_path: &str, apk_path: &str) -> anyhow::Result<()> {
    Command::new(adb_path)
        .args(["install", "-r", &apk_path])
        .output()?;
    Ok(())
}

//////////
// Devices

// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/transport.cpp;l=1398
#[derive(Debug)]
pub struct Device {
    device: String,
    model: String,
    path: String,
    product: String,
    serial: String,
}

pub fn list_devices<B>(adb_path: &str) -> anyhow::Result<B>
where
    B: FromIterator<Device>,
{
    let output = Command::new(adb_path).args(["devices", "-l"]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let devices = text.lines().filter_map(parse_device).collect();
    Ok(devices)
}

fn parse_device(line: &str) -> Option<Device> {
    let mut slices = line.split_whitespace();
    let serial = slices.next();
    let path = slices.next();
    let product = parse_device_pair(slices.next()?);
    let model = parse_device_pair(slices.next()?);
    let device = parse_device_pair(slices.next()?);
    if let (Some(serial), Some(path), Some(product), Some(model), Some(device)) =
        (serial, path, product, model, device)
    {
        Some(Device {
            serial: serial.to_owned(),
            path: path.to_owned(),
            product,
            model,
            device,
        })
    } else {
        None
    }
}

fn parse_device_pair(pair: &str) -> Option<String> {
    let mut slice = pair.split(":");
    let _key = slice.next();
    if let Some(value) = slice.next() {
        Some(value.to_string())
    } else {
        None
    }
}

////////
// Paths

/// Returns the path of a local (i.e. installed by ALVR) or OS version of `adb` if found, `None` otherwise.
pub fn get_adb_path() -> Option<String> {
    get_os_adb_path().or(get_local_adb_path())
}

fn get_os_adb_path() -> Option<String> {
    let path = ADB_EXECUTABLE.to_owned();
    if Command::new(&path).output().is_ok() {
        Some(path)
    } else {
        None
    }
}

fn get_local_adb_path() -> Option<String> {
    let path = get_platform_tools_path().ok()?.join(ADB_EXECUTABLE);
    if path.try_exists().is_ok_and(|e| e) {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

fn get_platform_tools_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::current_dir()?.join("platform-tools"))
}

//////////////////
// Port forwarding

#[derive(Debug)]
pub struct ForwardedPort {
    serial: String,
    local: u16,
    remote: u16,
}

pub fn list_forwarded_ports<B>(adb_path: &str) -> anyhow::Result<B>
where
    B: FromIterator<ForwardedPort>,
{
    let output = Command::new(adb_path)
        .args(["forward", "--list"])
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let forwarded_ports = text.lines().filter_map(parse_forwarded_port).collect();
    Ok(forwarded_ports)
}

fn parse_forwarded_port(line: &str) -> Option<ForwardedPort> {
    let mut slices = line.split_whitespace();
    let serial = slices.next();
    let local = parse_port_with_protocol(slices.next()?);
    let remote = parse_port_with_protocol(slices.next()?);
    if let (Some(serial), Some(local), Some(remote)) = (serial, local, remote) {
        Some(ForwardedPort {
            serial: serial.to_owned(),
            local,
            remote,
        })
    } else {
        None
    }
}

fn parse_port_with_protocol(value: &str) -> Option<u16> {
    let mut slices = value.split(":");
    let _protocol = slices.next();
    let maybe_port = slices.next();
    if let Some(port) = maybe_port {
        port.parse::<u16>().ok()
    } else {
        None
    }
}

pub fn forward_port(adb_path: &str, port: u16) -> anyhow::Result<()> {
    Command::new(adb_path)
        .args([
            "forward",
            &format!("tcp:{}", port),
            &format!("tcp:{}", port),
        ])
        .output()?;
    Ok(())
}
