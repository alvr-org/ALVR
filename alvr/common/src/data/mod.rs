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

use logging::LogId;
pub use settings::*;
pub use version::*;

type SettingsCache = SettingsDefault;

pub const SESSION_FNAME: &str = "session.json";

pub fn load_session(path: &Path) -> StrResult<SessionDesc> {
    trace_err!(json::from_str(&trace_err!(fs::read_to_string(path))?))
}

pub fn save_session(session_desc: &SessionDesc, path: &Path) -> StrResult {
    trace_err!(fs::write(
        path,
        trace_err!(json::to_string_pretty(session_desc))?
    ))
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
    pub version: String,
    pub revert_confirm_dialog: bool,
    pub restart_confirm_dialog: bool,
    pub last_clients: Vec<ClientConnectionDesc>,
    pub settings_cache: SettingsCache,
}

impl Default for SessionDesc {
    fn default() -> Self {
        Self {
            version: ALVR_SERVER_VERSION.into(),
            setup_wizard: true,
            revert_confirm_dialog: true,
            restart_confirm_dialog: true,
            last_clients: vec![],
            settings_cache: settings_cache_default(),
        }
    }
}

impl SessionDesc {
    // If json_value is not a valid representation of SessionDesc (because of version upgrade), use
    // some fuzzy logic to extrapolate as much information as possible.
    // Since SessionDesc cannot have a schema (because SettingsCache would need to also have a
    // schema, but it is generated out of our control), I only do basic name checking on fields and
    // deserialization will fail if the type of values does not match. Because of this,
    // `settings_cache` must be handled separately to do a better job of retrieving data using the
    // settings schema.
    pub fn merge_from_json(&mut self, json_value: json::Value) -> StrResult {
        const SETTINGS_CACHE_STR: &str = "settings_cache";

        if let Ok(session_desc) = json::from_value(json_value.clone()) {
            *self = session_desc;
            return Ok(());
        }

        let old_session_json = json::to_value(SessionDesc::default()).unwrap();
        let old_session_fields = old_session_json.as_object().unwrap();
        let new_session_fields = json_value.as_object().unwrap();

        let settings_cache_json = new_session_fields
            .get(SETTINGS_CACHE_STR)
            .map(|new_cache_json| {
                extrapolate_settings_cache(
                    &old_session_json[SETTINGS_CACHE_STR],
                    new_cache_json,
                    &settings_schema(settings_cache_default()),
                )
            })
            .unwrap_or_else(|| json_value.clone());

        let new_fields = old_session_fields
            .iter()
            .map(|(name, json_value)| {
                let new_json_value = if name == SETTINGS_CACHE_STR {
                    json::to_value(settings_cache_default()).unwrap()
                } else {
                    new_session_fields.get(name).unwrap_or(json_value).clone()
                };
                (name.clone(), new_json_value)
            })
            .collect();
        // Failure to extrapolate other session_desc fields is not notified.
        let mut session_desc_mut =
            json::from_value::<SessionDesc>(json::Value::Object(new_fields)).unwrap_or_default();

        match json::from_value::<SettingsCache>(settings_cache_json) {
            Ok(settings_cache) => {
                session_desc_mut.settings_cache = settings_cache;
                *self = session_desc_mut;
            }
            Err(e) => {
                *self = session_desc_mut;
                return trace_str!(
                    id: LogId::SettingsCacheExtrapolationFailed,
                    "Error while deserializing extrapolated settings cache: {}",
                    e
                );
            }
        }

        Ok(())
    }
}

// Current data extrapolation strategy: match both field name and value type exactly.
// Integer bounds are not validated, if they do not match the schema, deserialization will fail and
// all data is lost.
// Future strategies: check if value respects schema constraints, fuzzy field name matching, accept
// integer to float and float to integer, tree traversal.
fn extrapolate_settings_cache(
    old_cache: &json::Value,
    new_cache: &json::Value,
    schema: &SchemaNode,
) -> json::Value {
    match schema {
        SchemaNode::Section { entries } => json::Value::Object(
            entries
                .iter()
                .filter_map(|(field_name, maybe_data)| {
                    maybe_data.as_ref().map(|data_schema| {
                        let value_json = if let Some(new_value_json) = new_cache.get(field_name) {
                            extrapolate_settings_cache(
                                &old_cache[field_name],
                                new_value_json,
                                &data_schema.content,
                            )
                        } else {
                            old_cache[field_name].clone()
                        };
                        (field_name.clone(), value_json)
                    })
                })
                .collect(),
        ),

        SchemaNode::Choice { variants, .. } => {
            let variant_json = new_cache
                .get("variant")
                .cloned()
                .filter(|new_variant_json| {
                    new_variant_json
                        .as_str()
                        .map(|variant_str| {
                            variants
                                .iter()
                                .any(|(variant_name, _)| variant_str == variant_name)
                        })
                        .is_some()
                })
                .unwrap_or_else(|| old_cache["variant"].clone());

            let mut fields: json::Map<_, _> = variants
                .iter()
                .filter_map(|(variant_name, maybe_data)| {
                    maybe_data.as_ref().map(|data_schema| {
                        let value_json = if let Some(new_value_json) = new_cache.get(variant_name) {
                            extrapolate_settings_cache(
                                &old_cache[variant_name],
                                new_value_json,
                                &data_schema.content,
                            )
                        } else {
                            old_cache[variant_name].clone()
                        };
                        (variant_name.clone(), value_json)
                    })
                })
                .collect();
            fields.insert("variant".into(), variant_json);

            json::Value::Object(fields)
        }

        SchemaNode::Optional { content, .. } => {
            let set_json = new_cache
                .get("set")
                .cloned()
                .filter(|new_set_json| new_set_json.is_boolean())
                .unwrap_or_else(|| old_cache["set"].clone());

            let content_json = new_cache
                .get("content")
                .map(|new_content_json| {
                    extrapolate_settings_cache(&old_cache["content"], new_content_json, content)
                })
                .unwrap_or_else(|| old_cache["content"].clone());

            json::json!({
                "set": set_json,
                "content": content_json
            })
        }

        SchemaNode::Switch { content, .. } => {
            let enabled_json = new_cache
                .get("enabled")
                .cloned()
                .filter(|new_enabled_json| new_enabled_json.is_boolean())
                .unwrap_or_else(|| old_cache["enabled"].clone());

            let content_json = new_cache
                .get("content")
                .map(|new_content_json| {
                    extrapolate_settings_cache(&old_cache["content"], new_content_json, content)
                })
                .unwrap_or_else(|| old_cache["content"].clone());

            json::json!({
                "enabled": enabled_json,
                "content": content_json
            })
        }

        SchemaNode::Boolean { .. } => {
            if new_cache.is_boolean() {
                new_cache.clone()
            } else {
                old_cache.clone()
            }
        }

        SchemaNode::Integer { .. } => {
            if new_cache.is_i64() {
                new_cache.clone()
            } else {
                old_cache.clone()
            }
        }

        SchemaNode::Float { .. } => {
            if new_cache.is_f64() {
                new_cache.clone()
            } else {
                old_cache.clone()
            }
        }

        SchemaNode::Text { .. } => {
            if new_cache.is_string() {
                new_cache.clone()
            } else {
                old_cache.clone()
            }
        }

        SchemaNode::Array(array_schema) => {
            let array_vec = (0..array_schema.len())
                .map(|idx| {
                    new_cache
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| old_cache[idx].clone())
                })
                .collect();
            json::Value::Array(array_vec)
        }

        SchemaNode::Vector {
            default_element, ..
        } => {
            let element_json = new_cache
                .get("element")
                .map(|new_element_json| {
                    extrapolate_settings_cache(
                        &old_cache["content"],
                        new_element_json,
                        default_element,
                    )
                })
                .unwrap_or_else(|| old_cache["content"].clone());

            // todo: default field cannot be properly validated until I implement plain settings
            // validation (not to be confused with session/settings_cache validation). Any
            // problem inside this new_cache default will result in the loss all data in session.
            let default = new_cache
                .get("default")
                .cloned()
                .unwrap_or_else(|| old_cache["default"].clone());

            json::json!({
                "element": element_json,
                "default": default
            })
        }

        SchemaNode::Dictionary { default_value, .. } => {
            let key_json = new_cache
                .get("key")
                .cloned()
                .filter(|new_key| new_key.is_string())
                .unwrap_or_else(|| old_cache["key"].clone());

            let value_json = new_cache
                .get("value")
                .map(|new_value_json| {
                    extrapolate_settings_cache(&old_cache["value"], new_value_json, default_value)
                })
                .unwrap_or_else(|| old_cache["content"].clone());

            // todo: validate default using settings validation
            let default = new_cache
                .get("default")
                .cloned()
                .unwrap_or_else(|| old_cache["default"].clone());

            json::json!({
                "key": key_json,
                "value": value_json,
                "default": default
            })
        }
    }
}

// This function requires that settings enums with data have tag = "type" and content = "content", and
// enums without data do not have tag and content set.
pub fn session_to_settings(session: &SessionDesc) -> Settings {
    let cache_json = json::to_value(&session.settings_cache).unwrap();
    let schema = settings_schema(settings_cache_default());
    json::from_value(json_cache_to_settings(&cache_json, &schema)).unwrap()
}

fn json_cache_to_settings(cache: &json::Value, schema: &SchemaNode) -> json::Value {
    match schema {
        SchemaNode::Section { entries } => json::Value::Object(
            entries
                .iter()
                .filter_map(|(field_name, maybe_data)| {
                    maybe_data.as_ref().map(|data_schema| {
                        (
                            field_name.clone(),
                            json_cache_to_settings(&cache[field_name], &data_schema.content),
                        )
                    })
                })
                .collect(),
        ),

        SchemaNode::Choice { variants, .. } => {
            let variant = cache["variant"].clone();
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
                        json_cache_to_settings(&cache[variant], &data_schema.content)
                    });
                json::json!({
                    "type": variant,
                    "content": maybe_content
                })
            }
        }

        SchemaNode::Optional { content, .. } => {
            if cache["set"].as_bool().unwrap() {
                json_cache_to_settings(&cache["content"], content)
            } else {
                json::Value::Null
            }
        }

        SchemaNode::Switch { content, .. } => {
            let state;
            let maybe_content;
            if cache["enabled"].as_bool().unwrap() {
                state = "enabled";
                maybe_content = Some(json_cache_to_settings(&cache["content"], content))
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
        | SchemaNode::Text { .. } => cache.clone(),

        SchemaNode::Array(array_schema) => json::Value::Array(
            array_schema
                .iter()
                .enumerate()
                .map(|(idx, element_schema)| {
                    json_cache_to_settings(&cache[idx], element_schema)
                })
                .collect(),
        ),

        SchemaNode::Vector { .. } | SchemaNode::Dictionary { .. } => cache["default"].clone(),
    }
}

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
        save_session(self.session_desc, &self.dir.join(SESSION_FNAME)).ok();
    }
}

pub struct SessionManager {
    session_desc: SessionDesc,
    dir: PathBuf,
}

impl SessionManager {
    pub fn new(dir: &Path) -> Self {
        let session_desc = match fs::read_to_string(dir.join(SESSION_FNAME)) {
            Ok(session_string) => {
                let json_value = json::from_str::<json::Value>(&session_string).unwrap();
                match json::from_value(json_value.clone()) {
                    Ok(session_desc) => session_desc,
                    Err(_) => {
                        fs::write(dir.join("session_old.json"), &session_string).ok();
                        let mut session_desc = SessionDesc::default();
                        session_desc.merge_from_json(json_value).unwrap();
                        warn!(
                            "Session extrapolated. Old session.json is stored as session_old.json"
                        );
                        session_desc
                    }
                }
            }
            Err(_) => SessionDesc::default(),
        };

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

    // todo: add more tests
    #[test]
    fn test_session_extrapolation_trivial() {
        SessionDesc::default()
            .merge_from_json(json::to_value(SessionDesc::default()).unwrap())
            .unwrap();
    }
}
