//! Where the emulator keeps what belongs to the user rather than to the app.
//!
//! An installed app cannot write next to its own executable (an MSIX package's folder is
//! read-only), so the settings files and the user's own room live in a per-user folder:
//! `%LOCALAPPDATA%\ALVR Client Emulator` on Windows, `~/.local/share/ALVR Client Emulator` on
//! Linux. What ships with the app, the bundled room and models, stays next to the executable.

use alvr_common::{info, warn};
use std::path::{Path, PathBuf};

const USER_DIR_NAME: &str = "ALVR Client Emulator";

/// The per-user folder, created if it does not exist yet. Falls back to the executable's folder
/// on a system with no per-user data folder.
pub fn user_dir() -> PathBuf {
    let Some(dir) = dirs::data_local_dir().map(|dir| dir.join(USER_DIR_NAME)) else {
        return executable_dir();
    };

    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("Cannot create {}: {e}", dir.display());
    }

    dir
}

/// The folder the executable is in, where the build puts what ships with the app.
pub fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Copies a settings file found next to the executable into the user folder, if the user folder
/// has none yet, so changes made to it there carry over to where settings are kept.
pub fn adopt_settings_file(name: &str) {
    let source = executable_dir().join(name);
    let target = user_dir().join(name);

    if target.exists() || !source.exists() {
        return;
    }

    match std::fs::copy(&source, &target) {
        Ok(_) => info!("Copied {} to {}", source.display(), target.display()),
        Err(e) => warn!(
            "Cannot copy {} to {}: {e}",
            source.display(),
            target.display()
        ),
    }
}
