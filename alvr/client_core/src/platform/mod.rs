// #[cfg(target_os = "android")]
mod android;

// #[cfg(target_os = "android")]
use android::*;

use alvr_common::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

// #[cfg(target_os = "android")]
pub use android::{context, device_name, load_asset, try_get_microphone_permission, vm};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub protocol_id: u64,
    pub hostname: String,
    pub dark_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut rng = rand::thread_rng();

        Self {
            protocol_id: alvr_common::protocol_id(),
            hostname: format!(
                "{}{}{}{}.client.alvr",
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
            ),
            dark_mode: false,
        }
    }
}

pub fn load_config() -> Config {
    match serde_json::from_str(&load_config_string()) {
        Ok(config) => config,
        Err(_) => {
            // Failure happens if the Config signature changed between versions.
            // todo: recover data from mismatched Config signature. low priority
            info!("Error loading ALVR config. Using default");

            let config = Config::default();
            store_config(&config);

            config
        }
    }
}

pub fn store_config(config: &Config) {
    store_config_string(serde_json::to_string(config).unwrap())
}
