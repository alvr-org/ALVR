// https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md

use crate::parse::{self, Device, ForwardedPorts};
use alvr_filesystem as afs;
use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashSet,
    io::{Cursor, Read},
    process::Command,
    time::Duration,
};
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

fn get_command(adb_path: &str, args: &[&str]) -> Command {
    let mut command = Command::new(adb_path);
    command.args(args);

    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    command
}

pub fn download(url: &str, progress_callback: impl Fn(usize, Option<usize>)) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into();
    let response = agent.get(url).call()?;
    let maybe_expected_size = response
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok()?.parse::<usize>().ok());
    let mut result = maybe_expected_size
        .map(Vec::with_capacity)
        .unwrap_or_default();
    let mut reader = response.into_body().into_reader();
    let mut buffer = [0; 65535];
    loop {
        let read_count: usize = reader.read(&mut buffer)?;
        if read_count == 0 {
            break;
        }
        result.extend_from_slice(&buffer[..read_count]);
        let current_size = result.len();
        (progress_callback)(current_size, maybe_expected_size);
    }

    Ok(result)
}

///////////
// Activity

pub fn get_process_id(
    adb_path: &str,
    device_serial: &str,
    process_name: &str,
) -> Result<Option<usize>> {
    let output = get_command(
        adb_path,
        &["-s", device_serial, "shell", "pidof", process_name],
    )
    .output()
    .context(format!("Failed to get ID of process {process_name}"))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        return Ok(None);
    }
    let process_id = text
        .parse::<usize>()
        .context("Failed to parse process ID")?;

    Ok(Some(process_id))
}

pub fn is_activity_resumed(
    adb_path: &str,
    device_serial: &str,
    activity_name: &str,
) -> Result<bool> {
    let output = get_command(
        adb_path,
        &[
            "-s",
            device_serial,
            "shell",
            "dumpsys",
            "activity",
            activity_name,
        ],
    )
    .output()
    .context(format!("Failed to get state of activity {activity_name}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = text
        .lines()
        .map(|l| l.trim())
        .find(|l| l.contains("mResumed"))
    {
        let (entry, _) = line
            .split_once(' ')
            .ok_or(anyhow!("Failed to split resumed state line"))?;
        let (_, value) = entry
            .split_once('=')
            .ok_or(anyhow!("Failed to split resumed state entry"))?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(anyhow!("Failed to parse resumed state value"))?,
        }
    } else {
        Err(anyhow!("Failed to find resumed state line"))
    }
}

///////////////////
// ADB Installation

pub fn require_adb(
    layout: &afs::Layout,
    progress_callback: impl Fn(usize, Option<usize>),
) -> Result<String> {
    match get_adb_path(layout) {
        Some(path) => Ok(path),
        None => {
            install_adb(layout, progress_callback).context("Failed to install ADB")?;
            let path = get_adb_path(layout).context("Failed to get ADB path after installation")?;
            Ok(path)
        }
    }
}

fn install_adb(
    layout: &afs::Layout,
    progress_callback: impl Fn(usize, Option<usize>),
) -> Result<()> {
    let mut reader = Cursor::new(download_adb(progress_callback)?);
    ZipArchive::new(&mut reader)?.extract(layout.executables_dir.clone())?;

    Ok(())
}

fn download_adb(progress_callback: impl Fn(usize, Option<usize>)) -> Result<Vec<u8>> {
    let url = get_platform_tools_url();

    download(&url, progress_callback).context(format!("Failed to download ADB from {url}"))
}

fn get_platform_tools_url() -> String {
    format!("https://dl.google.com/android/repository/platform-tools{PLATFORM_TOOLS_VERSION}-{PLATFORM_TOOLS_OS}.zip")
}

///////////////
// Applications

pub fn start_application(adb_path: &str, device_serial: &str, application_id: &str) -> Result<()> {
    get_command(
        adb_path,
        &[
            "-s",
            device_serial,
            "shell",
            "monkey",
            "-p",
            application_id,
            "1",
        ],
    )
    .output()
    .context(format!("Failed to start {application_id}"))?;

    Ok(())
}

//////////
// Devices

pub fn list_devices(adb_path: &str) -> Result<Vec<Device>> {
    let output = get_command(adb_path, &["devices", "-l"])
        .output()
        .context("Failed to list ADB devices")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let devices = text
        .lines()
        .skip(1)
        .filter_map(parse::parse_device)
        .collect();

    Ok(devices)
}

///////////
// Packages

pub fn install_package(adb_path: &str, device_serial: &str, apk_path: &str) -> Result<()> {
    get_command(adb_path, &["-s", device_serial, "install", "-r", apk_path])
        .output()
        .context(format!("Failed to install {apk_path}"))?;

    Ok(())
}

pub fn is_package_installed(
    adb_path: &str,
    device_serial: &str,
    application_id: &str,
) -> Result<bool> {
    let found = list_installed_packages(adb_path, device_serial)
        .context(format!(
            "Failed to check if package {application_id} is installed"
        ))?
        .contains(application_id);

    Ok(found)
}

pub fn uninstall_package(adb_path: &str, device_serial: &str, application_id: &str) -> Result<()> {
    get_command(
        adb_path,
        &["-s", device_serial, "uninstall", application_id],
    )
    .output()
    .context(format!("Failed to uninstall {application_id}"))?;

    Ok(())
}

pub fn list_installed_packages(adb_path: &str, device_serial: &str) -> Result<HashSet<String>> {
    let output = get_command(
        adb_path,
        &["-s", device_serial, "shell", "pm", "list", "package"],
    )
    .output()
    .context("Failed to list installed packages")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let packages = text.lines().map(|l| l.replace("package:", "")).collect();

    Ok(packages)
}

////////
// Paths

/// Returns the path of a local (i.e. installed by ALVR) or OS version of `adb` if found, `None` otherwise.
pub fn get_adb_path(layout: &afs::Layout) -> Option<String> {
    let exe_name = afs::exec_fname("adb").to_owned();
    let adb_path = get_command(&exe_name, &[])
        .output()
        .is_ok()
        .then_some(exe_name);

    adb_path.or_else(|| {
        let path = layout.local_adb_exe();

        path.try_exists()
            .unwrap_or(false)
            .then(|| path.to_string_lossy().to_string())
    })
}

//////////////////
// Port forwarding

pub fn list_forwarded_ports(adb_path: &str, device_serial: &str) -> Result<Vec<ForwardedPorts>> {
    let output = get_command(adb_path, &["-s", device_serial, "forward", "--list"])
        .output()
        .context(format!(
            "Failed to list forwarded ports of device {device_serial:?}"
        ))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let forwarded_ports = text
        .lines()
        .filter_map(parse::parse_forwarded_ports)
        .collect();

    Ok(forwarded_ports)
}

pub fn forward_port(adb_path: &str, device_serial: &str, port: u16) -> Result<()> {
    get_command(
        adb_path,
        &[
            "-s",
            device_serial,
            "forward",
            &format!("tcp:{}", port),
            &format!("tcp:{}", port),
        ],
    )
    .output()
    .context(format!(
        "Failed to forward port {port:?} of device {device_serial:?}"
    ))?;

    Ok(())
}

/////////
// Server

pub fn kill_server(adb_path: &str) -> Result<()> {
    get_command(adb_path, &["kill-server"])
        .output()
        .context("Failed to kill ADB server")?;

    Ok(())
}
