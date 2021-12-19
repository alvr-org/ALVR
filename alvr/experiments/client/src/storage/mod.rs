#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod desktop;

#[cfg(target_os = "android")]
use android::*;
#[cfg(not(target_os = "android"))]
use desktop::*;

use alvr_common::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: format!("{}.client.alvr", rand::random::<u16>()),
        }
    }
}

pub fn load_config() -> StrResult<Config> {
    let maybe_config = serde_json::from_str(&load_config_string());

    if let Ok(config) = maybe_config {
        Ok(config)
    } else {
        let config = Config::default();
        store_config(&config)?;

        Ok(config)
    }
}

pub fn store_config(config: &Config) -> StrResult {
    store_config_string(trace_err!(serde_json::to_string(config))?);

    Ok(())
}
