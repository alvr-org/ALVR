//! Running the dashboard from an MSIX package.
//!
//! On Windows the streamer is portable: the dashboard, the SteamVR driver, `session.json` and the
//! logs all live in one folder, and the driver finds that folder from its own location. An MSIX
//! package's folder is read-only and its name changes with every version, so neither the settings
//! nor the driver SteamVR loads can stay there. When the dashboard runs from a package it copies
//! the streamer files to a fixed per-user folder and uses that as its root instead, so SteamVR
//! keeps loading the driver from the same path across updates and the settings and logs land
//! somewhere writable.
//!
//! The package manifest excludes that folder (and `%LOCALAPPDATA%\openvr`, where drivers are
//! registered) from MSIX file system virtualization. Without that, the dashboard's writes there
//! would go to a private per-package copy that SteamVR never sees.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The folder the streamer files are copied to, under `%LOCALAPPDATA%`.
const STREAMER_DIR: &str = "ALVR\\streamer";

/// Records which package the files in the streamer folder were copied from, so they are only
/// copied again after an install or an update.
const MARKER_FILE: &str = "msix_package.txt";

/// Files that belong to the user rather than to the package. Never overwritten, and a package
/// does not ship them anyway.
const USER_FILES: &[&str] = &["session.json", "session_log.txt", "crash_log.txt"];

/// What happened while preparing the streamer folder, for logging once logging is up.
pub struct Prepared {
    pub root: PathBuf,
    pub messages: Vec<Result<String, String>>,
}

/// The full name of the package this process runs from, or `None` when it is not packaged.
pub fn package_full_name() -> Option<String> {
    use windows_sys::Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER, Storage::Packaging::Appx::GetCurrentPackageFullName,
    };

    let mut length = 0u32;
    // SAFETY: a null buffer with a zero length only queries the required length.
    let result = unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) };
    if result != ERROR_INSUFFICIENT_BUFFER {
        // APPMODEL_ERROR_NO_PACKAGE: an ordinary, unpackaged install.
        return None;
    }

    let mut buffer = vec![0u16; length as usize];
    // SAFETY: the buffer holds `length` UTF-16 units, as the call above asked for.
    let result = unsafe { GetCurrentPackageFullName(&mut length, buffer.as_mut_ptr()) };
    if result != 0 {
        return None;
    }

    // `length` includes the terminating null.
    buffer.truncate(length.saturating_sub(1) as usize);
    Some(String::from_utf16_lossy(&buffer))
}

/// When running from a package, brings the per-user streamer folder up to date with the package
/// and returns it. Returns `None` when not packaged, and the dashboard then uses its own folder.
///
/// `package_root` is the folder the dashboard executable is in, which holds the streamer files.
pub fn prepare_streamer_dir(package_root: &Path) -> Option<Prepared> {
    let package = package_full_name()?;
    let root = dirs::data_local_dir()?.join(STREAMER_DIR);
    let mut messages = vec![];

    let marker = root.join(MARKER_FILE);
    let up_to_date = fs::read_to_string(&marker).is_ok_and(|copied| copied.trim() == package);

    if !up_to_date {
        match copy_tree(package_root, &root) {
            Ok(()) => {
                fs::write(&marker, &package).ok();
                messages.push(Ok(format!(
                    "Copied the streamer files of {package} to {}",
                    root.display()
                )));
            }
            // Most likely SteamVR still has the old driver loaded. The marker is left alone, so
            // the copy is tried again the next time the dashboard starts.
            Err(e) => messages.push(Err(format!(
                "Cannot update the streamer files in {}: {e}. Close SteamVR and restart the \
                 dashboard to finish updating.",
                root.display()
            ))),
        }
    }

    Some(Prepared { root, messages })
}

/// Copies every file from `source` into `destination`, overwriting what the package ships and
/// leaving everything else there, such as the user's settings and logs, alone.
fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = destination.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_tree(&path, &target)?;
        } else if !USER_FILES.iter().any(|name| entry.file_name() == *name) {
            fs::copy(&path, &target)?;
        }
    }

    Ok(())
}
