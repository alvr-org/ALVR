use alvr_common::{error, info};
use app_dirs2::{AppDataType, AppInfo};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

fn config_path() -> PathBuf {
    app_dirs2::app_root(
        AppDataType::UserConfig,
        &AppInfo {
            name: "ALVR Client",
            author: "ALVR",
        },
    )
    .unwrap()
    .join("session.json")
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub protocol_id: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut rng = rand::thread_rng();

        Self {
            hostname: format!(
                "{}{}{}{}.client",
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
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
