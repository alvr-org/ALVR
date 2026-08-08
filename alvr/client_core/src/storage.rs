use alvr_common::{error, info};
use app_dirs2::{AppDataType, AppInfo};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

fn app_root() -> PathBuf {
    app_dirs2::app_root(
        AppDataType::UserConfig,
        &AppInfo {
            name: "ALVR Client",
            author: "ALVR",
        },
    )
    .unwrap()
}

fn config_path() -> PathBuf {
    app_root().join("session.json")
}

/// Directory for the alpha stream debug texture dumps.
///
/// On Android this deliberately uses the app-specific *external* files dir rather than the private
/// data dir: release APKs are not debuggable, so `adb run-as` cannot read private storage, but
/// `/sdcard/Android/data/<package>/files` is readable over adb and needs no runtime permission.
#[cfg(target_os = "android")]
pub fn debug_dump_dir() -> PathBuf {
    // The process name is the package name on Android, which avoids hardcoding a build variant.
    let package = fs::read("/proc/self/cmdline")
        .ok()
        .and_then(|raw| {
            let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
            String::from_utf8(raw[..end].to_vec()).ok()
        })
        .unwrap_or_else(|| alvr_system_info::PACKAGE_NAME_GITHUB_DEV.to_owned());

    let dir = PathBuf::from(format!("/sdcard/Android/data/{package}/files/alpha_debug"));

    // Fall back to private storage if external storage is unavailable for any reason.
    if fs::create_dir_all(&dir).is_ok() {
        dir
    } else {
        error!("Alpha debug: cannot use external storage, falling back to private dir");
        app_root().join("alpha_debug")
    }
}

#[cfg(not(target_os = "android"))]
pub fn debug_dump_dir() -> PathBuf {
    app_root().join("alpha_debug")
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub protocol_id: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut rng = rand::rng();

        Self {
            hostname: format!(
                "{}{}{}{}.client.local.",
                rng.random_range(0..10),
                rng.random_range(0..10),
                rng.random_range(0..10),
                rng.random_range(0..10),
            ),
            protocol_id: alvr_common::protocol_id(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Ok(config_string) = fs::read_to_string(config_path()) {
            // Failure happens if the Config signature changed between versions.
            // todo: recover data from mismatched Config signature. low priority
            if let Ok(config) = serde_json::from_str(&config_string) {
                return config;
            } else {
                info!("Error parsing ALVR config. Using default");
            }
        } else {
            info!("Error reading ALVR config. Using default");
        }

        let config = Config::default();
        config.store();

        config
    }

    pub fn store(&self) {
        let config_string = serde_json::to_string(self).unwrap();
        if let Err(e) = fs::write(config_path(), config_string) {
            error!("Error writing ALVR config: {e}")
        }
    }
}
