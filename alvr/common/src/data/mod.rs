mod version;
mod settings;

use serde::{Deserialize, Serialize};

pub use version::*;
pub use settings::*;

type SettingsCache = SettingsDefault;

pub const SESSION_FNAME: &str = "session.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHandshake {
    // todo
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDesc {
    pub setup_wizard: bool,
    pub revert_confirm_dialog: bool,
    pub last_clients: Vec<ClientHandshake>,
    pub settings_cache: SettingsCache,
}

impl Default for SessionDesc {
    fn default() -> Self {
        Self {
            setup_wizard: true,
            revert_confirm_dialog: true,
            last_clients: vec![],
            // todo: recreate from settings file if available
            settings_cache: settings_default(),
        }
    }
}
