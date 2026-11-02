use super::schema::{HigherOrderChoiceSchema, PresetModifierOperation};
use crate::dashboard::components::{self, NestingInfo, SettingControl};
use alvr_common::debug;
use alvr_packets::{PathSegment, PathValuePair};
use eframe::egui::Ui;
use serde_json as json;
use settings_schema::{SchemaEntry, SchemaNode};
use std::collections::{HashMap, HashSet};

pub struct Control {
    name: String,
    modifiers: HashMap<String, Vec<PathValuePair>>,
    control: SettingControl,
    preset_json: json::Value,
}

impl Control {
    pub fn new(schema: HigherOrderChoiceSchema) -> Self {
        let modifiers = schema
            .options
            .iter()
            .map(|option| {
                (
                    option.display_name.clone(),
                    option
                        .modifiers
                        .iter()
                        .map(|modifier| match &modifier.operation {
                            PresetModifierOperation::Assign(value) => PathValuePair {
                                path: alvr_common::show_err(alvr_packets::parse_path(
                                    &modifier.target_path,
                                ))
                                .unwrap_or_default(),
                                value: value.clone(),
                            },
                        })
                        .collect(),
                )
            })
            .collect();

        let mut strings = schema.strings;
        strings.insert("display_name".into(), schema.name.clone());

        let control_schema = SchemaNode::Section {
            entries: vec![SchemaEntry {
                name: schema.name.clone(),
                strings,
                flags: schema.flags,
                content: SchemaNode::Choice {
                    default: schema
                        .options
                        .iter()
                        .find(|option| option.display_name == schema.default_option_display_name)
                        .unwrap()
                        .display_name
                        .clone(),
                    variants: schema
                        .options
                        .into_iter()
                        .map(|option| SchemaEntry {
                            name: option.display_name.clone(),
                            strings: [("display_name".into(), option.display_name)]
                                .into_iter()
                                .collect(),
                            flags: HashSet::new(),
                            content: None,
                        })
                        .collect(),
                    gui: Some(schema.gui),
                },
            }],
            gui_collapsible: false,
        };

        let control = SettingControl::new(
            NestingInfo {
                path: vec![],
                indentation_level: 0,
            },
            control_schema,
        );

        let preset_json = json::json!({ {&schema.name}: { "variant": "" } });

        Self {
            name: schema.name,
            modifiers,
            control,
            preset_json,
        }
    }

    pub fn update_session_settings(&mut self, session_setting_json: &json::Value) {
        let mut selected_option = String::new();

        'outer: for (key, descs) in &self.modifiers {
            for desc in descs {
                let mut session_ref = session_setting_json;

                // Note: the first path segment should be "settings_schema". Skip that.
                let Some(segments) = desc.path.get(1..) else {
                    continue 'outer;
                };
                for segment in segments {
                    let next = match segment {
                        PathSegment::Name(name) => session_ref.get(name),
                        PathSegment::Index(index) => session_ref.get(index),
                    };
                    let Some(next) = next else {
                        debug!(
                            "Preset modifier targets \"{}\", which is not in the current session",
                            alvr_packets::path_to_string(&desc.path)
                        );
                        continue 'outer;
                    };
                    session_ref = next;
                }

                if !components::json_values_eq(session_ref, &desc.value) {
                    continue 'outer;
                }
            }

            // At this point the session matches all modifiers
            selected_option.clone_from(key);

            break;
        }

        // Note: if no modifier matched, the control will unselect all options
        self.preset_json[&self.name]["variant"] = json::Value::String(selected_option);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<PathValuePair> {
        if let Some(desc) = self.control.ui(ui, &mut self.preset_json, false) {
            // todo: handle children requests
            self.modifiers[desc.value.as_str().unwrap()].clone()
        } else {
            vec![]
        }
    }
}
