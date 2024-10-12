// https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md

pub mod connection_state;
pub mod device;
pub mod forwarded_port;
pub mod transport_type;

use std::{io::Cursor, path::PathBuf, process::Command, time::Duration};

use const_format::formatcp;
use device::Device;
use forwarded_port::ForwardedPort;
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

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

///////////////////
// ADB Installation

pub fn install_adb() -> anyhow::Result<()> {
    let buffer = download_adb()?;
    let mut reader = Cursor::new(buffer);
    let path = get_installation_path()?;
    ZipArchive::new(&mut reader)?.extract(path)?;
    Ok(())
}

fn download_adb() -> anyhow::Result<Vec<u8>> {
    let response = ureq::get(PLATFORM_TOOLS_URL)
        .timeout(REQUEST_TIMEOUT)
        .call()?;
    let mut buffer = Vec::<u8>::new();
    response.into_reader().read_to_end(&mut buffer)?;
    Ok(buffer)
}

///////////////////
// APK installation

pub fn install_apk(adb_path: &str, device_serial: &str, apk_path: &str) -> anyhow::Result<()> {
    Command::new(adb_path)
        .args(["-s", &device_serial, "install", "-r", &apk_path])
        .output()?;
    Ok(())
}

//////////
// Devices

pub fn list_devices(adb_path: &str) -> anyhow::Result<Vec<Device>> {
    let output = Command::new(adb_path).args(["devices", "-l"]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let devices = text.lines().filter_map(device::parse).collect();
    Ok(devices)
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

fn get_installation_path() -> anyhow::Result<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop();
    Ok(path)
}

fn get_platform_tools_path() -> anyhow::Result<PathBuf> {
    Ok(get_installation_path()?.join("platform-tools"))
}

//////////////////
// Port forwarding

pub fn list_forwarded_ports(
    adb_path: &str,
    device_serial: &str,
) -> anyhow::Result<Vec<ForwardedPort>> {
    let output = Command::new(adb_path)
        .args(["-s", &device_serial, "forward", "--list"])
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let forwarded_ports = text.lines().filter_map(forwarded_port::parse).collect();
    Ok(forwarded_ports)
}

pub fn forward_port(adb_path: &str, device_serial: &str, port: u16) -> anyhow::Result<()> {
    Command::new(adb_path)
        .args([
            "-s",
            &device_serial,
            "forward",
            &format!("tcp:{}", port),
            &format!("tcp:{}", port),
        ])
        .output()?;
    Ok(())
}
