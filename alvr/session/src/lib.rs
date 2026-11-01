mod settings;

pub use settings::*;
pub use settings_schema;

use alvr_common::{
    ALVR_VERSION, ConnectionState, ToAny,
    anyhow::{Result, bail},
    semver::Version,
};
use serde::{Deserialize, Serialize};
use serde_json as json;
use settings_schema::{NumberType, SchemaNode};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};

// SessionSettings is similar to Settings but it contains every branch, even unused ones. This is
// the settings representation that the UI uses.
pub type SessionSettings = settings::SettingsDefault;

// Stores client-negotiated streaming parameters that are needed by C++ before
// InitializeStreaming() is called (HMD device registration). Settings-derived fields are read
// directly from the settings section of session.json and do not need to be duplicated here.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SteamvrHmdInitConfig {
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub target_eye_resolution_width: u32,
    pub target_eye_resolution_height: u32,
    pub refresh_rate: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientConnectionConfig {
    pub display_name: String,
    pub current_ip: Option<IpAddr>,
    pub manual_ips: HashSet<IpAddr>,
    pub trusted: bool,
    pub connection_state: ConnectionState,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionConfig {
    pub server_version: Version,
    // The hashmap key is the hostname
    pub client_connections: HashMap<String, ClientConnectionConfig>,
    pub session_settings: SessionSettings,
    pub restart_settings_hash: u64,
    pub steamvr_hmd_init_config: SteamvrHmdInitConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            server_version: ALVR_VERSION.clone(),
            client_connections: HashMap::new(),
            session_settings: settings::session_settings_default(),
            restart_settings_hash: 0,
            steamvr_hmd_init_config: SteamvrHmdInitConfig {
                // avoid realistic resolutions, as on first start, on Linux, it
                // could trigger direct mode on an existing monitor
                eye_resolution_width: 800,
                eye_resolution_height: 900,
                target_eye_resolution_width: 800,
                target_eye_resolution_height: 900,
                refresh_rate: 60,
            },
        }
    }
}

impl SessionConfig {
    // If json_value is not a valid representation of SessionConfig (because of version upgrade),
    // use some fuzzy logic to extrapolate as much information as possible.
    // Since SessionConfig cannot have a schema (because SessionSettings would need to also have a
    // schema, but it is generated out of our control), we only do basic name checking on fields and
    // deserialization will fail if the type of values does not match. Because of this,
    // `session_settings` must be handled separately to do a better job of retrieving data using the
    // settings schema.
    pub fn merge_from_json(&mut self, json_value: &json::Value) -> Result<()> {
        const SESSION_SETTINGS_STR: &str = "session_settings";

        if let Ok(session_desc) = json::from_value(json_value.clone()) {
            *self = session_desc;
            return Ok(());
        }

        // Note: unwrap is safe because current session is expected to serialize correctly
        let old_session_json = json::to_value(self.clone()).unwrap();
        let old_session_fields = old_session_json.as_object().unwrap();

        let maybe_session_settings_json =
            json_value
                .get(SESSION_SETTINGS_STR)
                .map(|new_session_settings_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_fields[SESSION_SETTINGS_STR],
                        new_session_settings_json,
                        &Settings::schema(settings::session_settings_default()),
                    )
                });

        let new_fields = old_session_fields
            .iter()
            .map(|(name, json_field_value)| {
                let new_json_field_value = if name == SESSION_SETTINGS_STR {
                    json::to_value(settings::session_settings_default()).unwrap()
                } else {
                    json_value.get(name).unwrap_or(json_field_value).clone()
                };
                (name.clone(), new_json_field_value)
            })
            .collect();
        // Failure to extrapolate other session_desc fields is not notified.
        let mut session_desc_mut =
            json::from_value::<SessionConfig>(json::Value::Object(new_fields)).unwrap_or_default();

        match maybe_session_settings_json
            .to_any()
            .and_then(|s| serde_json::from_value::<SessionSettings>(s).map_err(|e| e.into()))
        {
            Ok(session_settings) => {
                session_desc_mut.session_settings = session_settings;
                *self = session_desc_mut;
                Ok(())
            }
            Err(e) => {
                *self = session_desc_mut;

                bail!("Error while deserializing extrapolated session settings: {e}")
            }
        }
    }

    pub fn to_settings(&self) -> Settings {
        let session_settings_json = json::to_value(&self.session_settings).unwrap();
        let schema = Settings::schema(settings::session_settings_default());

        json::from_value::<Settings>(json_session_settings_to_settings(
            &session_settings_json,
            &schema,
        ))
        .map_err(|e| dbg!(e))
        .unwrap()
    }
}

// Current data extrapolation strategy: match both field name and value type exactly.
// Integer bounds are not validated, if they do not match the schema, deserialization will fail and
// all data is lost.
// Future strategies: check if value respects schema constraints, fuzzy field name matching, accept
// integer to float and float to integer, tree traversal.
fn extrapolate_session_settings_from_session_settings(
    old_session_settings: &json::Value,
    new_session_settings: &json::Value,
    schema: &SchemaNode,
) -> json::Value {
    match schema {
        SchemaNode::Section {
            entries,
            gui_collapsible,
        } => json::Value::Object({
            let mut entries: json::Map<String, json::Value> = entries
                .iter()
                .map(|named_entry| {
                    let value_json = extrapolate_session_settings_from_session_settings(
                        &old_session_settings[&named_entry.name],
                        &new_session_settings[&named_entry.name],
                        &named_entry.content,
                    );
                    (named_entry.name.clone(), value_json)
                })
                .collect();

            if *gui_collapsible {
                let collapsed_json = if new_session_settings["gui_collapsed"].is_boolean() {
                    new_session_settings["gui_collapsed"].clone()
                } else {
                    old_session_settings["gui_collapsed"].clone()
                };
                entries.insert("gui_collapsed".into(), collapsed_json);
            }

            entries
        }),

        SchemaNode::Choice { variants, .. } => {
            let variant_json = json::from_value(new_session_settings["variant"].clone())
                .ok()
                .filter(|variant_str| {
                    variants
                        .iter()
                        .any(|named_entry| *variant_str == named_entry.name)
                })
                .map_or_else(
                    || old_session_settings["variant"].clone(),
                    json::Value::String,
                );

            let mut fields: json::Map<_, _> = variants
                .iter()
                .filter_map(|named_entry| {
                    named_entry.content.as_ref().map(|data_schema| {
                        let value_json = extrapolate_session_settings_from_session_settings(
                            &old_session_settings[&named_entry.name],
                            &new_session_settings[&named_entry.name],
                            data_schema,
                        );
                        (named_entry.name.clone(), value_json)
                    })
                })
                .collect();
            fields.insert("variant".into(), variant_json);

            json::Value::Object(fields)
        }

        SchemaNode::Optional { content, .. } => {
            let set_value = new_session_settings["set"]
                .as_bool()
                .unwrap_or_else(|| old_session_settings["set"].as_bool().unwrap());

            let content_json = extrapolate_session_settings_from_session_settings(
                &old_session_settings["content"],
                &new_session_settings["content"],
                content,
            );

            json::json!({
                "set": set_value,
                "content": content_json
            })
        }

        SchemaNode::Switch { content, .. } => {
            let enabled = new_session_settings["enabled"]
                .as_bool()
                .unwrap_or_else(|| old_session_settings["enabled"].as_bool().unwrap());

            let content_json = extrapolate_session_settings_from_session_settings(
                &old_session_settings["content"],
                &new_session_settings["content"],
                content,
            );

            json::json!({
                "enabled": enabled,
                "content": content_json
            })
        }

        SchemaNode::Boolean { .. } => {
            if new_session_settings.is_boolean() {
                new_session_settings.clone()
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Number { ty, .. } => {
            if let Some(value) = new_session_settings.as_f64() {
                match ty {
                    NumberType::UnsignedInteger => json::Value::from(value.abs() as u64),
                    NumberType::SignedInteger => json::Value::from(value as i64),
                    NumberType::Float => new_session_settings.clone(),
                }
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Text { .. } => {
            if new_session_settings.is_string() {
                new_session_settings.clone()
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Array(array_schema) => {
            let gui_collapsed = new_session_settings["gui_collapsed"]
                .as_bool()
                .unwrap_or_else(|| old_session_settings["gui_collapsed"].as_bool().unwrap());

            let array_vec = array_schema
                .iter()
                .enumerate()
                .map(|(idx, schema)| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_settings["content"][idx],
                        &new_session_settings["content"][idx],
                        schema,
                    )
                })
                .collect::<Vec<_>>();

            json::json!({
                "gui_collapsed": gui_collapsed,
                "content": array_vec
            })
        }

        SchemaNode::Vector {
            default_element, ..
        } => {
            let gui_collapsed = new_session_settings["gui_collapsed"]
                .as_bool()
                .unwrap_or_else(|| old_session_settings["gui_collapsed"].as_bool().unwrap());

            let element_json = extrapolate_session_settings_from_session_settings(
                &old_session_settings["element"],
                &new_session_settings["element"],
                default_element,
            );

            let content_json =
                json::from_value::<Vec<json::Value>>(new_session_settings["content"].clone())
                    .ok()
                    .map(|vec| {
                        vec.iter()
                            .enumerate()
                            .map(|(idx, new_element)| {
                                extrapolate_session_settings_from_session_settings(
                                    &old_session_settings["content"]
                                        .get(idx)
                                        .cloned()
                                        .unwrap_or_else(|| element_json.clone()),
                                    new_element,
                                    default_element,
                                )
                            })
                            .collect()
                    })
                    .map_or_else(
                        || old_session_settings["content"].clone(),
                        json::Value::Array,
                    );

            json::json!({
                "gui_collapsed": gui_collapsed,
                "element": element_json,
                "content": content_json
            })
        }

        SchemaNode::Dictionary { default_value, .. } => {
            let gui_collapsed = new_session_settings["gui_collapsed"]
                .as_bool()
                .unwrap_or_else(|| old_session_settings["gui_collapsed"].as_bool().unwrap());

            let key_str = new_session_settings["key"]
                .as_str()
                .unwrap_or_else(|| old_session_settings["key"].as_str().unwrap());

            let value_json = extrapolate_session_settings_from_session_settings(
                &old_session_settings["value"],
                &new_session_settings["value"],
                default_value,
            );

            let content_json = json::from_value::<HashMap<String, json::Value>>(
                new_session_settings["content"].clone(),
            )
            .ok()
            .map(|map| {
                map.iter()
                    .map(|(key, new_value)| {
                        let value = extrapolate_session_settings_from_session_settings(
                            &old_session_settings["content"]
                                .get(key)
                                .cloned()
                                .unwrap_or_else(|| value_json.clone()),
                            new_value,
                            default_value,
                        );
                        (key, value)
                    })
                    .map(|(key, value)| {
                        json::Value::Array(vec![json::Value::String(key.clone()), value])
                    })
                    .collect()
            })
            .map_or_else(
                || old_session_settings["content"].clone(),
                json::Value::Array,
            );

            json::json!({
                "gui_collapsed": gui_collapsed,
                "key": key_str,
                "value": value_json,
                "content": content_json
            })
        }
        _ => unreachable!(),
    }
}

// session_settings does not get validated here, it must be already valid
fn json_session_settings_to_settings(
    session_settings: &json::Value,
    schema: &SchemaNode,
) -> json::Value {
    match schema {
        SchemaNode::Section { entries, .. } => json::Value::Object(
            entries
                .iter()
                .map(|named_entry| {
                    (
                        named_entry.name.clone(),
                        json_session_settings_to_settings(
                            &session_settings[&named_entry.name],
                            &named_entry.content,
                        ),
                    )
                })
                .collect(),
        ),

        SchemaNode::Choice { variants, .. } => {
            let variant = session_settings["variant"].as_str().unwrap();
            let maybe_content = variants
                .iter()
                .find(|named_entry| named_entry.name == variant)
                .and_then(|named_entry| named_entry.content.as_ref())
                .map(|data_schema| {
                    json_session_settings_to_settings(&session_settings[variant], data_schema)
                });
            if let Some(content) = maybe_content {
                json::json!({ variant: content })
            } else {
                json::Value::String(variant.to_owned())
            }
        }

        SchemaNode::Optional { content, .. } => {
            if session_settings["set"].as_bool().unwrap() {
                json_session_settings_to_settings(&session_settings["content"], content)
            } else {
                json::Value::Null
            }
        }

        SchemaNode::Switch { content, .. } => {
            if session_settings["enabled"].as_bool().unwrap() {
                let content =
                    json_session_settings_to_settings(&session_settings["content"], content);

                json::json!({ "Enabled": content })
            } else {
                json::Value::String("Disabled".into())
            }
        }

        SchemaNode::Boolean { .. } | SchemaNode::Number { .. } | SchemaNode::Text { .. } => {
            session_settings.clone()
        }

        SchemaNode::Array(array_schema) => json::Value::Array(
            array_schema
                .iter()
                .enumerate()
                .map(|(idx, element_schema)| {
                    json_session_settings_to_settings(
                        &session_settings["content"][idx],
                        element_schema,
                    )
                })
                .collect(),
        ),

        SchemaNode::Vector {
            default_element, ..
        } => json::to_value(
            session_settings["content"]
                .as_array()
                .unwrap()
                .iter()
                .map(|element_json| {
                    json_session_settings_to_settings(element_json, default_element)
                })
                .collect::<Vec<_>>(),
        )
        .unwrap(),

        SchemaNode::Dictionary { default_value, .. } => json::to_value(
            json::from_value::<Vec<(String, json::Value)>>(session_settings["content"].clone())
                .unwrap()
                .into_iter()
                .map(|(key, value_json)| {
                    (
                        key,
                        json_session_settings_to_settings(&value_json, default_value),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .unwrap(),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_session_to_settings() {
        let default = session_settings_default();
        let settings_schema = json::to_value(&default).unwrap();
        let schema = Settings::schema(default);

        let _settings = json::from_value::<Settings>(json_session_settings_to_settings(
            &settings_schema,
            &schema,
        ))
        .err();
    }

    #[test]
    fn test_session_to_settings() {
        let _settings = SessionConfig::default().to_settings();
    }

    #[test]
    fn test_session_extrapolation_trivial() {
        SessionConfig::default()
            .merge_from_json(&json::to_value(SessionConfig::default()).unwrap())
            .unwrap();
    }

    #[test]
    fn test_session_extrapolation_diff() {
        let input_json_string = r#"{
            "session_settings": {
              "fjdshfks": false,
              "video": {
                "preferred_fps": 60.0
              },
              "headset": {
                "gui_collapsed": false,
                "controllers": {
                  "enabled": false
                }
              }
            }
          }"#;

        let mut session = SessionConfig::default();
        session
            .merge_from_json(&json::from_str(input_json_string).unwrap())
            .unwrap();

        let settings = session.to_settings();

        assert_eq!(settings.video.preferred_fps, 60.0);
        assert!(settings.headset.controllers.as_option().is_none());
    }
}
