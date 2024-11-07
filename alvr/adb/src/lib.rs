// https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md

pub mod connection_state;
pub mod device;
pub mod forwarded_port;
pub mod transport_type;
pub mod wired_connection;

use std::{
    collections::HashSet, io::Cursor, path::PathBuf, process::Command, sync::OnceLock,
    time::Duration,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use alvr_common::{dbg_connection, once_cell::sync::Lazy, parking_lot::RwLock};
use alvr_filesystem::Layout;
use alvr_server_io::ServerSessionManager;
use anyhow::{anyhow, Context, Result};
use device::Device;
use forwarded_port::ForwardedPort;
use wired_connection::{WiredConnection, WiredConnectionStatus};
use zip::ZipArchive;

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

pub fn setup_wired_connection(
    filesystem_layout: &OnceLock<alvr_filesystem::Layout>,
    wired_connection: &mut WiredConnection,
    session_manager: &Lazy<RwLock<ServerSessionManager>>,
    control_port: u16,
    progress_callback: impl Fn(usize, Option<usize>),
) -> Result<()> {
    let layout = filesystem_layout
        .get()
        .context("Failed to get filesystem layout")?;
    let adb_path = require_adb(layout, progress_callback)?;
    wired_connection.maybe_adb_path = Some(adb_path.to_owned());

    let Some(device_serial) = list_devices(&adb_path)?
        .into_iter()
        .filter_map(|d| d.serial)
        .find(|s| !s.starts_with("127.0.0.1"))
    else {
        wired_connection.status =
            WiredConnectionStatus::NotReady("No wired devices found".to_owned());
        return Ok(());
    };

    let stream_port = session_manager.read().settings().connection.stream_port;
    let ports = HashSet::from([control_port, stream_port]);
    let forwarded_ports: HashSet<u16> = list_forwarded_ports(&adb_path, &device_serial)?
        .into_iter()
        .map(|f| f.local)
        .collect();
    let missing_ports = ports.difference(&forwarded_ports);
    for port in missing_ports {
        forward_port(&adb_path, &device_serial, *port)?;
        dbg_connection!("setup_wired_connection: Forwarded port {port} of device {device_serial}");
    }

    #[cfg(debug_assertions)]
    let process_name = "alvr.client.dev";
    #[cfg(not(debug_assertions))]
    let process_name = "alvr.client.stable";
    if get_process_id(&adb_path, &device_serial, process_name)?.is_none() {
        wired_connection.status =
            WiredConnectionStatus::NotReady("ALVR client is not running".to_owned());
        return Ok(());
    }
    if !is_activity_resumed(&adb_path, &device_serial, process_name)? {
        wired_connection.status =
            WiredConnectionStatus::NotReady("ALVR client is paused".to_owned());
        return Ok(());
    }

    wired_connection.status = WiredConnectionStatus::Ready;

    return Ok(());
}

fn get_command(adb_path: &str, args: &[&str]) -> Command {
    let mut command = Command::new(adb_path);
    command.args(args);

    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    command
}

fn download(url: &str, progress_callback: impl Fn(usize, Option<usize>)) -> Result<Vec<u8>> {
    let agent = ureq::builder()
        .timeout_connect(REQUEST_TIMEOUT)
        .timeout_read(REQUEST_TIMEOUT)
        .build();
    let response = agent.get(url).call()?;
    let maybe_expected_size: Option<usize> = response
        .header("Content-Length")
        .and_then(|l| l.parse().ok());
    let mut result = maybe_expected_size
        .map(|s| Vec::with_capacity(s))
        .unwrap_or_default();
    let mut reader = response.into_reader();
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
    layout: &Layout,
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

fn install_adb(layout: &Layout, progress_callback: impl Fn(usize, Option<usize>)) -> Result<()> {
    let buffer = download_adb(progress_callback)?;
    let mut reader = Cursor::new(buffer);
    let path = get_installation_path(layout);
    ZipArchive::new(&mut reader)?.extract(path)?;

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
    let devices = text.lines().skip(1).filter_map(device::parse).collect();

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
pub fn get_adb_path(layout: &Layout) -> Option<String> {
    get_os_adb_path().or(get_local_adb_path(layout))
}

fn get_os_adb_path() -> Option<String> {
    let name = get_executable_name().to_owned();

    get_command(&name, &[]).output().is_ok().then_some(name)
}

fn get_local_adb_path(layout: &Layout) -> Option<String> {
    let path = get_platform_tools_path(layout).join(get_executable_name());

    path.try_exists()
        .is_ok_and(|e| e)
        .then(|| path.to_string_lossy().to_string())
}

fn get_installation_path(layout: &Layout) -> PathBuf {
    layout.executables_dir.to_owned()
}

fn get_platform_tools_path(layout: &Layout) -> PathBuf {
    get_installation_path(layout).join("platform-tools")
}

fn get_executable_name() -> String {
    alvr_filesystem::exec_fname("adb")
}

//////////////////
// Port forwarding

fn list_forwarded_ports(adb_path: &str, device_serial: &str) -> Result<Vec<ForwardedPort>> {
    let output = get_command(adb_path, &["-s", device_serial, "forward", "--list"])
        .output()
        .context(format!(
            "Failed to list forwarded ports of device {device_serial:?}"
        ))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let forwarded_ports = text.lines().filter_map(forwarded_port::parse).collect();

    Ok(forwarded_ports)
}

fn forward_port(adb_path: &str, device_serial: &str, port: u16) -> Result<()> {
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
