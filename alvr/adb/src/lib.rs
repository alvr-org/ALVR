// https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md

pub mod connection_state;
pub mod device;
pub mod forwarded_port;
pub mod transport_type;

use std::{collections::HashSet, io::Cursor, path::PathBuf, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{Context, Result};
use device::Device;
use forwarded_port::ForwardedPort;
use zip::ZipArchive;

use alvr_common::{dbg_connection, warn};

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

pub fn setup_wired_connection() -> Result<()> {
    let adb_path = match get_adb_path() {
        Some(adb_path) => {
            dbg_connection!("Found ADB executable at {adb_path}");
            adb_path
        }
        None => {
            dbg_connection!("Couldn't find ADB, installing it...");
            install_adb(|downloaded, total| {
                let total_display = match total {
                    Some(t) => t.to_string(),
                    None => "?".to_owned(),
                };
                warn!("Downloading ADB: got {downloaded} bytes of {total_display}");
            })
            .context("Failed to install ADB")?;
            dbg_connection!("Finished installing ADB");
            let adb_path = get_adb_path().context("Failed to get ADB path after installation")?;
            dbg_connection!("ADB installed at {adb_path:?}");
            adb_path
        }
    };
    let devices = list_devices(&adb_path)?.into_iter().filter(|d| {
        d.serial
            .as_ref()
            .is_some_and(|s| !s.starts_with("127.0.0.1"))
    });
    let ports = HashSet::from([9943, 9944]);
    for device in devices {
        let Some(device_serial) = device.serial else {
            dbg_connection!("Skipping device without serial number");
            continue;
        };
        dbg_connection!("Forwarding ports {ports:?} of device {device_serial}...");
        forward_ports(&adb_path, &device_serial, &ports)?;
        dbg_connection!("Forwarded ports {ports:?} of device {device_serial}");
    }
    Ok(())
}

fn get_command(adb_path: &str, args: &[&str]) -> Command {
    let mut command = Command::new(adb_path);
    command.args(args);

    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    command
}

///////////////////
// ADB Installation

type ProgressCallback = fn(usize, Option<usize>);

pub fn install_adb(progress_callback: ProgressCallback) -> Result<()> {
    let buffer = download_adb(progress_callback)?;
    let mut reader = Cursor::new(buffer);
    let path = get_installation_path()?;
    ZipArchive::new(&mut reader)?.extract(path)?;
    Ok(())
}

fn download_adb(progress_callback: ProgressCallback) -> Result<Vec<u8>> {
    let url = get_platform_tools_url();
    let agent = ureq::builder()
        .timeout_connect(REQUEST_TIMEOUT)
        .timeout_read(REQUEST_TIMEOUT)
        .build();
    let response = agent.get(&url).call()?;
    let maybe_expected_size: Option<usize> = response
        .header("Content-Length")
        .map(|l| l.parse().ok())
        .flatten();
    let mut result = match maybe_expected_size {
        Some(size) => Vec::with_capacity(size),
        None => Vec::new(),
    };
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
            &device_serial,
            "shell",
            "monkey",
            "-p",
            &application_id,
            "1",
        ],
    )
    .output()
    .with_context(|| format!("Failed to start {application_id}"))?;
    Ok(())
}

///////////
// Packages

pub fn install_package(adb_path: &str, device_serial: &str, apk_path: &str) -> Result<()> {
    get_command(
        adb_path,
        &["-s", &device_serial, "install", "-r", &apk_path],
    )
    .output()
    .with_context(|| format!("Failed to install {apk_path}"))?;
    Ok(())
}

pub fn is_package_installed(
    adb_path: &str,
    device_serial: &str,
    application_id: &str,
) -> Result<bool> {
    let found = list_installed_packages(adb_path, device_serial)
        .with_context(|| format!("Failed to check if package {application_id} is installed"))?
        .contains(application_id);
    Ok(found)
}

pub fn replace_package(
    adb_path: &str,
    device_serial: &str,
    application_id: &str,
    apk_path: &str,
) -> Result<()> {
    if is_package_installed(adb_path, device_serial, application_id)? {
        uninstall_package(adb_path, device_serial, application_id)?;
    }
    install_package(adb_path, device_serial, apk_path)
}

pub fn uninstall_package(adb_path: &str, device_serial: &str, application_id: &str) -> Result<()> {
    get_command(
        adb_path,
        &["-s", &device_serial, "uninstall", &application_id],
    )
    .output()
    .with_context(|| format!("Failed to uninstall {application_id}"))?;
    Ok(())
}

pub fn list_installed_packages(adb_path: &str, device_serial: &str) -> Result<HashSet<String>> {
    let output = get_command(
        adb_path,
        &["-s", &device_serial, "shell", "pm", "list", "package"],
    )
    .output()
    .with_context(|| format!("Failed to list installed packages"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let packages = text.lines().map(|l| l.replace("package:", "")).collect();
    Ok(packages)
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

////////
// Paths

/// Returns the path of a local (i.e. installed by ALVR) or OS version of `adb` if found, `None` otherwise.
pub fn get_adb_path() -> Option<String> {
    get_os_adb_path().or(get_local_adb_path())
}

fn get_os_adb_path() -> Option<String> {
    let name = get_executable_name().to_owned();
    if get_command(&name, &[]).output().is_ok() {
        Some(name)
    } else {
        None
    }
}

fn get_local_adb_path() -> Option<String> {
    let path = get_platform_tools_path().ok()?.join(get_executable_name());
    if path.try_exists().is_ok_and(|e| e) {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

fn get_installation_path() -> Result<PathBuf> {
    let root = alvr_server_io::get_driver_dir_from_registered()?;
    let layout = alvr_filesystem::filesystem_layout_from_openvr_driver_root_dir(&root);
    Ok(layout.executables_dir)
}

fn get_platform_tools_path() -> Result<PathBuf> {
    Ok(get_installation_path()?.join("platform-tools"))
}

fn get_executable_name() -> String {
    alvr_filesystem::exec_fname("adb")
}

//////////////////
// Port forwarding

fn list_forwarded_ports(adb_path: &str, device_serial: &str) -> Result<Vec<ForwardedPort>> {
    let output = get_command(adb_path, &["-s", &device_serial, "forward", "--list"])
        .output()
        .with_context(|| format!("Failed to list forwarded ports of device {device_serial:?}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let forwarded_ports = text.lines().filter_map(forwarded_port::parse).collect();
    Ok(forwarded_ports)
}

fn forward_ports(adb_path: &str, device_serial: &str, ports: &HashSet<u16>) -> Result<()> {
    let forwarded_ports: HashSet<u16> = list_forwarded_ports(&adb_path, &device_serial)?
        .into_iter()
        .map(|f| f.local)
        .collect();
    let missing_ports = ports.difference(&forwarded_ports);
    for port in missing_ports {
        forward_port(&adb_path, &device_serial, *port)?;
    }
    Ok(())
}

fn forward_port(adb_path: &str, device_serial: &str, port: u16) -> Result<()> {
    get_command(
        adb_path,
        &[
            "-s",
            &device_serial,
            "forward",
            &format!("tcp:{}", port),
            &format!("tcp:{}", port),
        ],
    )
    .output()
    .with_context(|| format!("Failed to forward port {port:?} of device {device_serial:?}"))?;
    Ok(())
}
