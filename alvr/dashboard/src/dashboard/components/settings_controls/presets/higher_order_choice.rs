use std::collections::{HashMap, HashSet};

use super::schema::{HigherOrderChoiceSchema, PresetModifierOperation};
use crate::dashboard::components::{self, NestingInfo, SettingControl};
use alvr_gui_common::theme::{
    log_colors::{INFO_LIGHT, WARNING_LIGHT},
    OK_GREEN,
};
use alvr_packets::{PathSegment, PathValuePair};
use eframe::egui::{self, popup, Ui};
use serde_json as json;
use settings_schema::{SchemaEntry, SchemaNode};

const POPUP_ID: &str = "setpopup";

pub struct Control {
    name: String,
    help: Option<String>,
    steamvr_restart_flag: bool,
    real_time_flag: bool,
    modifiers: HashMap<String, Vec<PathValuePair>>,
    control: SettingControl,
    preset_json: json::Value,
}

impl Control {
    pub fn new(schema: HigherOrderChoiceSchema) -> Self {
        let name = components::get_display_name(&schema.name, &schema.strings);
        let help = schema.strings.get("help").cloned();
        // let notice = entry.strings.get("notice").cloned();
        let steamvr_restart_flag = schema.flags.contains("steamvr-restart");
        let real_time_flag = schema.flags.contains("real-time");

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

        let control_schema = SchemaNode::Choice {
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
        };

        let control = SettingControl::new(
            NestingInfo {
                path: vec![],
                indentation_level: 0,
            },
            control_schema,
        );

        let preset_json = json::json!({
            "variant": ""
        });

        Self {
            name,
            help,
            steamvr_restart_flag,
            real_time_flag,
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
        self.preset_json["variant"] = json::Value::String(selected_option);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<PathValuePair> {
        let mut response = None;

        ui.horizontal(|ui| {
            ui.label(&self.name);

            if let Some(string) = &self.help {
                if ui.colored_label(INFO_LIGHT, "❓").hovered() {
                    popup::show_tooltip_text(ui.ctx(), egui::Id::new(POPUP_ID), string);
                }
            }
            if self.steamvr_restart_flag && ui.colored_label(WARNING_LIGHT, "⚠").hovered() {
                popup::show_tooltip_text(
                    ui.ctx(),
                    egui::Id::new(POPUP_ID),
                    format!(
                        "Changing this setting will make SteamVR restart!\n{}",
                        "Please save your in-game progress first"
                    ),
                );
            }

            // The emoji is blue but it will be green in the UI
            if self.real_time_flag && ui.colored_label(OK_GREEN, "🔵").hovered() {
                popup::show_tooltip_text(
                    ui.ctx(),
                    egui::Id::new(POPUP_ID),
                    "This setting can be changed in real-time during streaming!",
                );
            }
        });
        response = self
            .control
            .ui(ui, &mut self.preset_json, true)
            .or(response);

        // ui.end_row();

        if let Some(desc) = response {
            // todo: handle children requests
            self.modifiers[desc.value.as_str().unwrap()].clone()
        } else {
            vec![]
        }
    }
}
