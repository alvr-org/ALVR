mod settings;

pub use settings::*;
pub use settings_schema;

use alvr_common::{prelude::*, semver::Version, ALVR_VERSION};
use serde::{Deserialize, Serialize};
use serde_json as json;
use settings_schema::{NumberType, SchemaNode};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    path::PathBuf,
};

// SessionSettings is similar to Settings but it contains every branch, even unused ones. This is
// the settings representation that the UI uses.
pub type SessionSettings = settings::SettingsDefault;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DriversBackup {
    pub alvr_path: PathBuf,
    pub other_paths: Vec<PathBuf>,
}

// This structure is used to store the minimum configuration data that ALVR driver needs to
// initialize OpenVR before having the chance to communicate with a client. When a client is
// connected, a new OpenvrConfig instance is generated, then the connection is accepted only if that
// instance is equivalent to the one stored in the session, otherwise SteamVR is restarted.
// Other components (like the encoder, audio recorder) don't need this treatment and are initialized
// dynamically.
// todo: properties that can be set after the OpenVR initialization should be removed and set with
// UpdateForStream.
#[derive(Serialize, Deserialize, PartialEq, Default, Clone, Debug)]
pub struct OpenvrConfig {
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub target_eye_resolution_width: u32,
    pub target_eye_resolution_height: u32,
    pub tracking_ref_only: bool,
    pub enable_vive_tracker_proxy: bool,
    pub aggressive_keyframe_resend: bool,
    pub adapter_index: u32,
    pub codec: u32,
    pub refresh_rate: u32,
    pub use_10bit_encoder: bool,
    pub enable_vbaq: bool,
    pub use_preproc: bool,
    pub preproc_sigma: u32,
    pub preproc_tor: u32,
    pub amd_encoder_quality_preset: u32,
    pub rate_control_mode: u32,
    pub filler_data: bool,
    pub entropy_coding: u32,
    pub force_sw_encoding: bool,
    pub sw_thread_count: u32,
    pub controllers_mode_idx: i32,
    pub controllers_enabled: bool,
    pub override_trigger_threshold: bool,
    pub trigger_threshold: f32,
    pub override_grip_threshold: bool,
    pub grip_threshold: f32,
    pub enable_foveated_rendering: bool,
    pub foveation_center_size_x: f32,
    pub foveation_center_size_y: f32,
    pub foveation_center_shift_x: f32,
    pub foveation_center_shift_y: f32,
    pub foveation_edge_ratio_x: f32,
    pub foveation_edge_ratio_y: f32,
    pub enable_color_correction: bool,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub sharpening: f32,
    pub linux_async_reprojection: bool,
    pub nvenc_quality_preset: u32,
    pub nvenc_tuning_preset: u32,
    pub nvenc_multi_pass: u32,
    pub nvenc_adaptive_quantization_mode: u32,
    pub nvenc_low_delay_key_frame_scale: i64,
    pub nvenc_refresh_rate: i64,
    pub enable_intra_refresh: bool,
    pub intra_refresh_period: i64,
    pub intra_refresh_count: i64,
    pub max_num_ref_frames: i64,
    pub gop_length: i64,
    pub p_frame_strategy: i64,
    pub nvenc_rate_control_mode: i64,
    pub rc_buffer_size: i64,
    pub rc_initial_delay: i64,
    pub rc_max_bitrate: i64,
    pub rc_average_bitrate: i64,
    pub nvenc_enable_weighted_prediction: bool,
    pub capture_frame_dir: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientConnectionDesc {
    pub display_name: String,
    pub current_ip: Option<IpAddr>,
    pub manual_ips: HashSet<IpAddr>,
    pub trusted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionDesc {
    pub server_version: Version,
    pub drivers_backup: Option<DriversBackup>,
    pub openvr_config: OpenvrConfig,
    // The hashmap key is the hostname
    pub client_connections: HashMap<String, ClientConnectionDesc>,
    pub session_settings: SessionSettings,
}

impl Default for SessionDesc {
    fn default() -> Self {
        Self {
            server_version: ALVR_VERSION.clone(),
            drivers_backup: None,
            openvr_config: OpenvrConfig {
                // avoid realistic resolutions, as on first start, on Linux, it
                // could trigger direct mode on an existing monitor
                eye_resolution_width: 800,
                eye_resolution_height: 900,
                target_eye_resolution_width: 800,
                target_eye_resolution_height: 900,
                adapter_index: 0,
                refresh_rate: 60,
                controllers_enabled: false,
                enable_foveated_rendering: false,
                enable_color_correction: false,
                linux_async_reprojection: false,
                capture_frame_dir: "/tmp".into(),
                ..<_>::default()
            },
            client_connections: HashMap::new(),
            session_settings: settings::session_settings_default(),
        }
    }
}

impl SessionDesc {
    // If json_value is not a valid representation of SessionDesc (because of version upgrade), use
    // some fuzzy logic to extrapolate as much information as possible.
    // Since SessionDesc cannot have a schema (because SessionSettings would need to also have a
    // schema, but it is generated out of our control), I only do basic name checking on fields and
    // deserialization will fail if the type of values does not match. Because of this,
    // `session_settings` must be handled separately to do a better job of retrieving data using the
    // settings schema.
    pub fn merge_from_json(&mut self, json_value: &json::Value) -> StrResult {
        const SESSION_SETTINGS_STR: &str = "session_settings";

        if let Ok(session_desc) = json::from_value(json_value.clone()) {
            *self = session_desc;
            return Ok(());
        }

        let old_session_json = json::to_value(&self).map_err(err!())?;
        let old_session_fields = old_session_json.as_object().ok_or_else(enone!())?;

        let maybe_session_settings_json =
            json_value
                .get(SESSION_SETTINGS_STR)
                .map(|new_session_settings_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_json[SESSION_SETTINGS_STR],
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
            json::from_value::<SessionDesc>(json::Value::Object(new_fields)).unwrap_or_default();

        match json::from_value::<SessionSettings>(maybe_session_settings_json.ok_or_else(enone!())?)
        {
            Ok(session_settings) => {
                session_desc_mut.session_settings = session_settings;
                *self = session_desc_mut;
                Ok(())
            }
            Err(e) => {
                *self = session_desc_mut;

                fmt_e!("Error while deserializing extrapolated session settings: {e}")
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
        SchemaNode::Section(entries) => json::Value::Object(
            entries
                .iter()
                .map(|named_entry| {
                    let value_json =
                        if let Some(new_value_json) = new_session_settings.get(&named_entry.name) {
                            extrapolate_session_settings_from_session_settings(
                                &old_session_settings[&named_entry.name],
                                new_value_json,
                                &named_entry.content,
                            )
                        } else {
                            old_session_settings[&named_entry.name].clone()
                        };
                    (named_entry.name.clone(), value_json)
                })
                .collect(),
        ),

        SchemaNode::Choice { variants, .. } => {
            let variant_json = new_session_settings
                .get("variant")
                .cloned()
                .filter(|new_variant_json| {
                    new_variant_json
                        .as_str()
                        .map(|variant_str| {
                            variants
                                .iter()
                                .any(|named_entry| variant_str == named_entry.name)
                        })
                        .is_some()
                })
                .unwrap_or_else(|| old_session_settings["variant"].clone());

            let mut fields: json::Map<_, _> = variants
                .iter()
                .filter_map(|named_entry| {
                    named_entry.content.as_ref().map(|data_schema| {
                        let value_json = if let Some(new_value_json) =
                            new_session_settings.get(&named_entry.name)
                        {
                            extrapolate_session_settings_from_session_settings(
                                &old_session_settings[&named_entry.name],
                                new_value_json,
                                data_schema,
                            )
                        } else {
                            old_session_settings[&named_entry.name].clone()
                        };
                        (named_entry.name.clone(), value_json)
                    })
                })
                .collect();
            fields.insert("variant".into(), variant_json);

            json::Value::Object(fields)
        }

        SchemaNode::Optional { content, .. } => {
            let set_json = new_session_settings
                .get("set")
                .cloned()
                .filter(|new_set_json| new_set_json.is_boolean())
                .unwrap_or_else(|| old_session_settings["set"].clone());

            let content_json = new_session_settings
                .get("content")
                .map(|new_content_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_settings["content"],
                        new_content_json,
                        content,
                    )
                })
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "set": set_json,
                "content": content_json
            })
        }

        SchemaNode::Switch { content, .. } => {
            let enabled_json = new_session_settings
                .get("enabled")
                .cloned()
                .filter(|new_enabled_json| new_enabled_json.is_boolean())
                .unwrap_or_else(|| old_session_settings["enabled"].clone());

            let content_json = new_session_settings
                .get("content")
                .map(|new_content_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_settings["content"],
                        new_content_json,
                        content,
                    )
                })
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "enabled": enabled_json,
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
            let array_vec = (0..array_schema.len())
                .map(|idx| {
                    new_session_settings
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| old_session_settings[idx].clone())
                })
                .collect();
            json::Value::Array(array_vec)
        }

        SchemaNode::Vector {
            default_element, ..
        } => {
            let element_json = new_session_settings
                .get("element")
                .map(|new_element_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_settings["element"],
                        new_element_json,
                        default_element,
                    )
                })
                .unwrap_or_else(|| old_session_settings["element"].clone());

            // todo: content field cannot be properly validated until I implement plain settings
            // validation (not to be confused with session/session_settings validation). Any
            // problem inside this new_session_settings content will result in the loss all data in the new
            // session_settings.
            let content_json = new_session_settings
                .get("content")
                .cloned()
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "element": element_json,
                "content": content_json
            })
        }

        SchemaNode::Dictionary { default_value, .. } => {
            let key_json = new_session_settings
                .get("key")
                .cloned()
                .filter(|new_key| new_key.is_string())
                .unwrap_or_else(|| old_session_settings["key"].clone());

            let value_json = new_session_settings
                .get("value")
                .map(|new_value_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_settings["value"],
                        new_value_json,
                        default_value,
                    )
                })
                .unwrap_or_else(|| old_session_settings["value"].clone());

            // todo: validate content using settings validation
            let content_json = new_session_settings
                .get("content")
                .cloned()
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "key": key_json,
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
        SchemaNode::Section(entries) => json::Value::Object(
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
                    json_session_settings_to_settings(&session_settings[idx], element_schema)
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
        let _settings = SessionDesc::default().to_settings();
    }

    #[test]
    fn test_session_extrapolation_trivial() {
        SessionDesc::default()
            .merge_from_json(&json::to_value(SessionDesc::default()).unwrap())
            .unwrap();
    }

    #[test]
    fn test_session_extrapolation_diff() {
        let input_json_string = r#"{
            "session_settings": {
              "fjdshfks":false,
              "video": {
                "preferred_fps": 60.0
              },
              "headset": {
                "controllers": {
                  "enabled": false
                }
              }
            }
          }"#;

        SessionDesc::default()
            .merge_from_json(&json::from_str(input_json_string).unwrap())
            .unwrap();
    }
}
