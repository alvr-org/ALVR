use super::{settings, Settings, DEFAULT_SESSION_SETTINGS, SETTINGS_SCHEMA};
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json as json;
use settings_schema::{
    EntryType, SchemaChoice, SchemaDictionary, SchemaNode, SchemaOptional, SchemaSwitch,
    SchemaVector,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::IpAddr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

pub const SESSION_FNAME: &str = "session.json";

// SessionSettings is similar to Settings but it contains every branch, even unused ones. This is
// the settings representation that the UI uses.
pub type SessionSettings = settings::SettingsDefault;

pub fn load_session(path: &Path) -> StrResult<SessionDesc> {
    trace_err!(json::from_str(&trace_err!(fs::read_to_string(path))?))
}

pub fn save_session(session_desc: &SessionDesc, path: &Path) -> StrResult {
    trace_err!(fs::write(
        path,
        trace_err!(json::to_string_pretty(session_desc))?
    ))
}

// This structure is used to store the minimum configuration data that ALVR driver needs to
// initialize OpenVR before having the chance to communicate with a client. When a client is
// connected, a new OpenvrConfig instance is generated, then the connection is accepted only if that
// instance is equivalent to the one stored in the session, otherwise SteamVR is restarted.
// Other components (like the encoder, audio recorder) don't need this treatment and are initialized
// dynamically.
// todo: properties that can be set after the OpenVR initialization should be removed and set with
// UpdateForStream.
#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct OpenvrConfig {
    pub universe_id: u64,
    pub headset_serial_number: String,
    pub headset_tracking_system_name: String,
    pub headset_model_number: String,
    pub headset_driver_version: String,
    pub headset_manufacturer_name: String,
    pub headset_render_model_name: String,
    pub headset_registered_device_type: String,
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub target_eye_resolution_width: u32,
    pub target_eye_resolution_height: u32,
    pub seconds_from_vsync_to_photons: f32,
    pub force_3dof: bool,
    pub aggressive_keyframe_resend: bool,
    pub adapter_index: u32,
    pub codec: u32,
    pub refresh_rate: u32,
    pub use_10bit_encoder: bool,
    pub encode_bitrate_mbs: u64,
    pub controllers_tracking_system_name: String,
    pub controllers_manufacturer_name: String,
    pub controllers_model_number: String,
    pub render_model_name_left_controller: String,
    pub render_model_name_right_controller: String,
    pub controllers_serial_number: String,
    pub controllers_type: String,
    pub controllers_registered_device_type: String,
    pub controllers_input_profile_path: String,
    pub controllers_mode_idx: i32,
    pub controllers_enabled: bool,
    pub position_offset: [f32; 3],
    pub tracking_frame_offset: i32,
    pub controller_pose_offset: f32,
    pub position_offset_left: [f32; 3],
    pub rotation_offset_left: [f32; 3],
    pub haptics_intensity: f32,
    pub enable_foveated_rendering: bool,
    pub foveation_strength: f32,
    pub foveation_shape: f32,
    pub foveation_vertical_offset: f32,
    pub enable_color_correction: bool,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub sharpening: f32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ClientConnectionDesc {
    pub display_name: String,
    pub manual_ips: HashSet<IpAddr>,
    pub trusted: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct SessionDesc {
    pub openvr_config: OpenvrConfig,
    // The hashmap key is the hostname
    pub client_connections: HashMap<String, ClientConnectionDesc>,
    pub session_settings: SessionSettings,
}

impl Default for SessionDesc {
    fn default() -> Self {
        Self {
            openvr_config: OpenvrConfig {
                universe_id: 2,
                headset_serial_number: "1WMGH000XX0000".into(),
                headset_tracking_system_name: "oculus".into(),
                headset_model_number: "Oculus Rift S".into(),
                headset_driver_version: "1.42.0".into(),
                headset_manufacturer_name: "Oculus".into(),
                headset_render_model_name: "generic_hmd".into(),
                headset_registered_device_type: "oculus/1WMGH000XX0000".into(),
                // avoid realistic resolutions, as on first start, on Linux, it
                // could trigger direct mode on an existing monitor
                eye_resolution_width: 800,
                eye_resolution_height: 900,
                target_eye_resolution_width: 800,
                target_eye_resolution_height: 900,
                seconds_from_vsync_to_photons: 0.005,
                adapter_index: 0,
                refresh_rate: 60,
                controllers_enabled: false,
                enable_foveated_rendering: false,
                enable_color_correction: false,
                ..<_>::default()
            },
            client_connections: HashMap::new(),
            session_settings: DEFAULT_SESSION_SETTINGS.clone(),
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

        let old_session_json = trace_err!(json::to_value(&self))?;
        let old_session_fields = trace_none!(old_session_json.as_object())?;

        let maybe_session_settings_json =
            json_value
                .get(SESSION_SETTINGS_STR)
                .map(|new_session_settings_json| {
                    extrapolate_session_settings_from_session_settings(
                        &old_session_json[SESSION_SETTINGS_STR],
                        new_session_settings_json,
                        &SETTINGS_SCHEMA,
                    )
                });

        let new_fields = old_session_fields
            .iter()
            .map(|(name, json_field_value)| {
                let new_json_field_value = if name == SESSION_SETTINGS_STR {
                    json::to_value(DEFAULT_SESSION_SETTINGS.clone()).unwrap()
                } else {
                    json_value.get(name).unwrap_or(json_field_value).clone()
                };
                (name.clone(), new_json_field_value)
            })
            .collect();
        // Failure to extrapolate other session_desc fields is not notified.
        let mut session_desc_mut =
            json::from_value::<SessionDesc>(json::Value::Object(new_fields)).unwrap_or_default();

        match json::from_value::<SessionSettings>(trace_none!(maybe_session_settings_json)?) {
            Ok(session_settings) => {
                session_desc_mut.session_settings = session_settings;
                *self = session_desc_mut;
                Ok(())
            }
            Err(e) => {
                *self = session_desc_mut;

                log_event(Event::SessionSettingsExtrapolationFailed);
                fmt_e!(
                    "Error while deserializing extrapolated session settings: {}",
                    e
                )
            }
        }
    }

    // This function requires that settings enums with data have tag = "type" and content = "content", and
    // enums without data do not have tag and content set.
    pub fn to_settings(&self) -> Settings {
        let session_settings_json = json::to_value(&self.session_settings).unwrap();
        json::from_value(json_session_settings_to_settings(
            &session_settings_json,
            &SETTINGS_SCHEMA,
        ))
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
                .filter_map(|(field_name, maybe_data)| {
                    if let EntryType::Data(data_schema) = maybe_data {
                        Some((field_name, data_schema))
                    } else {
                        None
                    }
                })
                .map(|(field_name, data_schema)| {
                    let value_json =
                        if let Some(new_value_json) = new_session_settings.get(field_name) {
                            extrapolate_session_settings_from_session_settings(
                                &old_session_settings[field_name],
                                new_value_json,
                                &data_schema.content,
                            )
                        } else {
                            old_session_settings[field_name].clone()
                        };
                    (field_name.clone(), value_json)
                })
                .collect(),
        ),

        SchemaNode::Choice(SchemaChoice { variants, .. }) => {
            let variant_json = new_session_settings
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
                .unwrap_or_else(|| old_session_settings["variant"].clone());

            let mut fields: json::Map<_, _> = variants
                .iter()
                .filter_map(|(variant_name, maybe_data)| {
                    maybe_data.as_ref().map(|data_schema| {
                        let value_json =
                            if let Some(new_value_json) = new_session_settings.get(variant_name) {
                                extrapolate_session_settings_from_session_settings(
                                    &old_session_settings[variant_name],
                                    new_value_json,
                                    &data_schema.content,
                                )
                            } else {
                                old_session_settings[variant_name].clone()
                            };
                        (variant_name.clone(), value_json)
                    })
                })
                .collect();
            fields.insert("variant".into(), variant_json);

            json::Value::Object(fields)
        }

        SchemaNode::Optional(SchemaOptional { content, .. }) => {
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

        SchemaNode::Switch(SchemaSwitch { content, .. }) => {
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

        SchemaNode::Boolean(_) => {
            if new_session_settings.is_boolean() {
                new_session_settings.clone()
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Integer(_) => {
            if new_session_settings.is_i64() {
                new_session_settings.clone()
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Float(_) => {
            if new_session_settings.is_number() {
                new_session_settings.clone()
            } else {
                old_session_settings.clone()
            }
        }

        SchemaNode::Text(_) => {
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

        SchemaNode::Vector(SchemaVector {
            default_element, ..
        }) => {
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

            // It's hard to recover data from a malformed dynamically sized array, if the order of
            // elements matters. Currently the content of new_session_settings is kept only if
            // it is well formed, otherwise it is completely replaced by old_session_settings.
            let content_json = new_session_settings
                .get("content")
                .map(|new_content_json| {
                    if let json::Value::Array(new_content_vec) = new_content_json {
                        let mut content_vec = vec![];
                        for new_content in new_content_vec {
                            let value = extrapolate_session_settings_from_session_settings(
                                &old_session_settings["element"],
                                new_content,
                                default_element,
                            );
                            content_vec.push(value);
                        }

                        if *new_content_vec == content_vec {
                            return json::Value::Array(content_vec);
                        }
                    }

                    old_session_settings["content"].clone()
                })
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "element": element_json,
                "content": content_json
            })
        }

        SchemaNode::Dictionary(SchemaDictionary { default_value, .. }) => {
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

            let content_json = new_session_settings
                .get("content")
                .map(|new_content_json| {
                    let maybe_entries =
                        json::from_value::<Vec<(String, json::Value)>>(new_content_json.clone());

                    if let Ok(new_entries) = maybe_entries {
                        let mut entries = vec![];
                        for (key, new_value) in &new_entries {
                            let value = extrapolate_session_settings_from_session_settings(
                                &old_session_settings["value"],
                                &new_value,
                                default_value,
                            );
                            entries.push((key.clone(), value));
                        }

                        if new_entries == entries {
                            return json::json!(new_entries);
                        }
                    }

                    old_session_settings["content"].clone()
                })
                .unwrap_or_else(|| old_session_settings["content"].clone());

            json::json!({
                "key": key_json,
                "value": value_json,
                "content": content_json
            })
        }
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
                .filter_map(|(field_name, maybe_data)| {
                    if let EntryType::Data(data_schema) = maybe_data {
                        Some((field_name, data_schema))
                    } else {
                        None
                    }
                })
                .map(|(field_name, data_schema)| {
                    (
                        field_name.clone(),
                        json_session_settings_to_settings(
                            &session_settings[field_name],
                            &data_schema.content,
                        ),
                    )
                })
                .collect(),
        ),

        SchemaNode::Choice(SchemaChoice { variants, .. }) => {
            let variant = session_settings["variant"].as_str().unwrap();
            let maybe_content = variants
                .iter()
                .find(|(variant_name, _)| variant_name == variant)
                .and_then(|(_, maybe_data)| maybe_data.as_ref())
                .map(|data_schema| {
                    json_session_settings_to_settings(
                        &session_settings[variant],
                        &data_schema.content,
                    )
                });
            json::json!({
                "type": variant,
                "content": maybe_content
            })
        }

        SchemaNode::Optional(SchemaOptional { content, .. }) => {
            if session_settings["set"].as_bool().unwrap() {
                json_session_settings_to_settings(&session_settings["content"], content)
            } else {
                json::Value::Null
            }
        }

        SchemaNode::Switch(SchemaSwitch { content, .. }) => {
            let state;
            let maybe_content;
            if session_settings["enabled"].as_bool().unwrap() {
                state = "Enabled";
                maybe_content = Some(json_session_settings_to_settings(
                    &session_settings["content"],
                    content,
                ))
            } else {
                state = "Disabled";
                maybe_content = None;
            }

            json::json!({
                "state": state,
                "content": maybe_content
            })
        }

        SchemaNode::Boolean(_)
        | SchemaNode::Integer(_)
        | SchemaNode::Float(_)
        | SchemaNode::Text(_) => session_settings.clone(),

        SchemaNode::Array(array_schema) => json::Value::Array(
            array_schema
                .iter()
                .enumerate()
                .map(|(idx, element_schema)| {
                    json_session_settings_to_settings(&session_settings[idx], element_schema)
                })
                .collect(),
        ),

        SchemaNode::Vector(SchemaVector {
            default_element, ..
        }) => json::Value::Array(
            session_settings["content"]
                .as_array()
                .unwrap()
                .iter()
                .map(|element| json_session_settings_to_settings(element, default_element))
                .collect(),
        ),

        SchemaNode::Dictionary(SchemaDictionary { default_value, .. }) => {
            let entries =
                json::from_value::<Vec<(String, json::Value)>>(session_settings["content"].clone())
                    .unwrap();

            let entries = entries
                .iter()
                .map(|(key, value)| (key, json_session_settings_to_settings(value, default_value)))
                .collect::<Vec<_>>();

            json::json!(entries)
        }
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
        log_event(Event::SessionUpdated);
    }
}

pub struct SessionManager {
    session_desc: SessionDesc,
    dir: PathBuf,
}

impl SessionManager {
    pub fn new(dir: &Path) -> Self {
        let session_path = dir.join(SESSION_FNAME);
        let session_desc = match fs::read_to_string(&session_path) {
            Ok(session_string) => {
                let json_value = json::from_str::<json::Value>(&session_string).unwrap();
                match json::from_value(json_value.clone()) {
                    Ok(session_desc) => session_desc,
                    Err(_) => {
                        fs::write(dir.join("session_old.json"), &session_string).ok();
                        let mut session_desc = SessionDesc::default();
                        match session_desc.merge_from_json(&json_value) {
                            Ok(_) => info!(
                                "{} {}",
                                "Session extrapolated successfully.",
                                "Old session.json is stored as session_old.json"
                            ),
                            Err(e) => error!(
                                "{} {} {}",
                                "Error while extrapolating session.",
                                "Old session.json is stored as session_old.json.",
                                e
                            ),
                        }
                        // not essential, but useful to avoid duplicated errors
                        save_session(&session_desc, &session_path).ok();

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

    pub fn get(&self) -> &SessionDesc {
        &self.session_desc
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
    fn test_schema() {
        println!("{:#?}", *SETTINGS_SCHEMA);
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
    fn test_session_extrapolation_oculus_go() {
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
