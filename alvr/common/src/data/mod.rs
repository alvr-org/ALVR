mod settings;
mod version;

use crate::*;
use serde::*;
use serde_json as json;
use settings_schema::SchemaNode;
use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

pub use settings::*;
pub use version::*;

type SettingsCache = SettingsDefault;

pub const SESSION_FNAME: &str = "session.json";

pub fn load_json<T: de::DeserializeOwned>(path: &Path) -> StrResult<T> {
    trace_err!(json::from_str(&trace_err!(fs::read_to_string(path))?))
}

pub fn save_json<T: Serialize>(obj: &T, path: &Path) -> StrResult {
    trace_err!(fs::write(path, trace_err!(json::to_string_pretty(obj))?))
}

#[repr(C)]
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientHandshakePacket {
    pub alvr_name: [u8; 4],
    pub version: [u8; 32],
    pub device_name: [u8; 32],
    pub client_refresh_rate: u16,
    pub render_width: u32,
    pub render_height: u32,
    pub client_fov: [Fov; 2],
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConnectionDesc {
    pub available: bool,
    pub connect_automatically: bool,
    pub last_update_ms_since_epoch: u64,
    pub address: String,
    pub handshake_packet: ClientHandshakePacket,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDesc {
    pub setup_wizard: bool,
    pub revert_confirm_dialog: bool,
    pub last_clients: Vec<ClientConnectionDesc>,
    pub settings_cache: SettingsCache,
}

impl Default for SessionDesc {
    fn default() -> Self {
        Self {
            setup_wizard: true,
            revert_confirm_dialog: true,
            last_clients: vec![],
            settings_cache: settings_cache_default(),
        }
    }
}

// This function requires that settings enums with data have tag = "type" and content="content", and
// enums without data do not have tag and content set.
// unwrap() calls never panic because SettingsCache structure is generated from Settings
pub fn session_to_settings(session: &SessionDesc) -> Settings {
    let cache_value = json::to_value(&session.settings_cache).unwrap();
    let schema = settings_schema(settings_cache_default());
    json::from_value(cache_to_settings_impl(&cache_value, &schema)).unwrap()
}

fn cache_to_settings_impl(cache_value: &json::Value, schema: &SchemaNode) -> json::Value {
    match schema {
        SchemaNode::Section { entries } => json::Value::Object(
            entries
                .iter()
                .filter_map(|(field_name, maybe_data)| {
                    maybe_data.as_ref().map(|data_schema| {
                        (
                            field_name.clone(),
                            cache_to_settings_impl(&cache_value[field_name], &data_schema.content),
                        )
                    })
                })
                .collect(),
        ),
        SchemaNode::Choice { variants, .. } => {
            let variant = cache_value["variant"].clone();
            let only_tag = variants
                .iter()
                .all(|(_, maybe_data)| matches!(maybe_data, None));
            if only_tag {
                variant
            } else {
                let variant = variant.as_str().unwrap();
                let maybe_content = variants
                    .iter()
                    .find(|(variant_name, _)| variant_name == variant)
                    .map(|(_, maybe_data)| maybe_data.as_ref())
                    .unwrap()
                    .map(|data_schema| {
                        cache_to_settings_impl(&cache_value[variant], &data_schema.content)
                    });
                json::json!({
                    "type": variant,
                    "content": maybe_content
                })
            }
        }
        SchemaNode::Optional { content, .. } => {
            if cache_value["set"].as_bool().unwrap() {
                cache_to_settings_impl(&cache_value["content"], content)
            } else {
                json::Value::Null
            }
        }
        SchemaNode::Switch { content, .. } => {
            let state;
            let maybe_content;
            if cache_value["enabled"].as_bool().unwrap() {
                state = "enabled";
                maybe_content = Some(cache_to_settings_impl(&cache_value["content"], content))
            } else {
                state = "disabled";
                maybe_content = None;
            }
            json::json!({
                "type": state,
                "content": maybe_content
            })
        }
        SchemaNode::Boolean { .. }
        | SchemaNode::Integer { .. }
        | SchemaNode::Float { .. }
        | SchemaNode::Text { .. } => cache_value.clone(),

        SchemaNode::Array(array_schema) => json::Value::Array(
            array_schema
                .iter()
                .enumerate()
                .map(|(idx, element_schema)| {
                    cache_to_settings_impl(&cache_value[idx], element_schema)
                })
                .collect(),
        ),
        SchemaNode::Vector { .. } | SchemaNode::Dictionary { .. } => cache_value["default"].clone(),
    }
}

// todo: settings_to_cache() -> useful for manual editing of settings

// SessionDesc wrapper that saves settings.json and session.json on destruction.
pub struct SessionLock<'a> {
    session_desc: &'a mut SessionDesc,
    dir: &'a Path,
}

impl Deref for SessionLock<'_> {
    type Target = SessionDesc;
    fn deref(&self) -> &SessionDesc {
        self.session_desc
    }
}

impl DerefMut for SessionLock<'_> {
    fn deref_mut(&mut self) -> &mut SessionDesc {
        self.session_desc
    }
}

impl Drop for SessionLock<'_> {
    fn drop(&mut self) {
        save_json(self.session_desc, &self.dir.join(SESSION_FNAME)).ok();
        save_json(
            &session_to_settings(self.session_desc),
            &self.dir.join(SETTINGS_FNAME),
        )
        .ok();
    }
}

pub struct SessionManager {
    session_desc: SessionDesc,
    dir: PathBuf,
}

impl SessionManager {
    pub fn new(dir: &Path) -> Self {
        let session_desc = load_json(&dir.join(SESSION_FNAME)).unwrap_or_default();
        Self {
            session_desc,
            dir: dir.to_owned(),
        }
    }

    pub fn get_mut(&mut self) -> SessionLock {
        SessionLock {
            session_desc: &mut self.session_desc,
            dir: &self.dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_to_settings() {
        let _settings = session_to_settings(&SessionDesc::default());
    }
}
