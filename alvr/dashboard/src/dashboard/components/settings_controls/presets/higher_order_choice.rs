use std::collections::{HashMap, HashSet};

use super::schema::{HigherOrderChoiceSchema, PresetModifierOperation};
use crate::dashboard::components::{NestingInfo, SettingControl};
use alvr_packets::{PathSegment, PathValuePair};
use eframe::egui::Ui;
use serde_json as json;
use settings_schema::{SchemaEntry, SchemaNode};

pub struct Control {
    name: String,
    modifiers: HashMap<String, Vec<PathValuePair>>,
    control: SettingControl,
    preset_json: json::Value,
}

impl Control {
    pub fn new(schema: HigherOrderChoiceSchema) -> Self {
        let name = schema.name.clone();

        // Compile PresetModifiers to ValueChangeDescs
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
                                path: alvr_packets::parse_path(&modifier.target_path),
                                value: value.clone(),
                            },
                        })
                        .collect(),
                )
            })
            .collect();

        let control_schema = SchemaNode::Section(
            [SchemaEntry {
                name: schema.name.clone(),
                strings: schema.strings,
                flags: schema.flags,
                content: SchemaNode::Choice {
                    default: schema.options[schema.default_option_index]
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
            }]
            .into_iter()
            .collect(),
        );
        let control = SettingControl::new(
            NestingInfo {
                path: vec![],
                indentation_level: 0,
            },
            control_schema,
        );

        let preset_json = json::json!({
            schema.name: {
                "variant": ""
            }
        });

        Self {
            name,
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

                // Note: the first path segment is always "settings_schema". Skip that.
                for segment in &desc.path[1..] {
                    session_ref = match segment {
                        PathSegment::Name(name) => {
                            if let Some(name) = session_ref.get(name) {
                                name
                            } else {
                                continue 'outer;
                            }
                        }
                        PathSegment::Index(index) => {
                            if let Some(index) = session_ref.get(index) {
                                index
                            } else {
                                continue 'outer;
                            }
                        }
                    };
                }

                if *session_ref != desc.value {
                    continue 'outer;
                }
            }

            // At this point the session matches all modifiers
            selected_option = key.clone();

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
