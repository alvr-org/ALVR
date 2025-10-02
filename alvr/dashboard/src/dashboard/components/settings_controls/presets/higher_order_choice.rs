use std::collections::{HashMap, HashSet};

use super::schema::HigherOrderChoiceSchema;
use crate::dashboard::components::{self, INDENTATION_STEP, NestingInfo, SettingControl, notice};
use alvr_gui_common::theme::{
    OK_GREEN,
    log_colors::{INFO_LIGHT, WARNING_LIGHT},
};
use alvr_packets::{Path, PathSegment, PathValuePair};
use eframe::egui::Ui;
use serde_json as json;
use settings_schema::{SchemaEntry, SchemaNode};

pub struct Control {
    name: String,
    help: Option<String>,
    notice: Option<String>,
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
        let notice = schema.strings.get("notice").cloned();
        let steamvr_restart_flag = schema.flags.contains("steamvr-restart");
        let real_time_flag = schema.flags.contains("real-time");

        let modifiers = schema
            .options
            .iter()
            .map(|option| (option.name.clone(), option.modifiers.0.clone()))
            .collect();
        let control_schema = SchemaNode::Choice {
            default: schema
                .options
                .iter()
                .find(|option| option.name == schema.default_option_name)
                .unwrap()
                .name
                .clone(),
            variants: schema
                .options
                .into_iter()
                .map(|option| SchemaEntry {
                    name: option.name.clone(),
                    strings: [("display_name".into(), option.name)].into_iter().collect(),
                    flags: HashSet::new(),
                    content: None,
                })
                .collect(),
            gui: Some(schema.gui),
        };

        let control = SettingControl::new(
            NestingInfo {
                path: Path(vec![]),
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
            notice,
            steamvr_restart_flag,
            real_time_flag,
            modifiers,
            control,
            preset_json,
        }
    }

    pub fn update_session_settings(&mut self, session_setting_json: &json::Value) {
        let mut selected_option = String::new();

        'outer: for (key, modifiers) in &self.modifiers {
            for modifier in modifiers {
                let mut session_ref = session_setting_json;

                // Note: the first path segment is always "settings_schema". Skip that.
                for segment in &modifier.path[1..] {
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

                if !components::json_values_eq(session_ref, &modifier.value) {
                    continue 'outer;
                }
            }

            // At this point the session matches all modifiers
            selected_option.clone_from(key);

            break;
        }

        // Note: if no modifier matched, the control will unselect all options
        self.preset_json["variant"] = json::Value::String(selected_option);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<PathValuePair> {
        let mut response = None;

        ui.horizontal(|ui| {
            ui.add_space(INDENTATION_STEP);
            ui.label(&self.name);

            if let Some(string) = &self.help
                && ui.colored_label(INFO_LIGHT, "❓").hovered()
            {
                alvr_gui_common::tooltip(ui, &format!("{}_help_tooltip", self.name), string);
            }
            if self.steamvr_restart_flag && ui.colored_label(WARNING_LIGHT, "⚠").hovered() {
                alvr_gui_common::tooltip(
                    ui,
                    "steamvr_restart_tooltip",
                    &format!(
                        "Changing this setting will make SteamVR restart!\n{}",
                        "Please save your in-game progress first"
                    ),
                );
            }

            // The emoji is blue but it will be green in the UI
            if self.real_time_flag && ui.colored_label(OK_GREEN, "🔵").hovered() {
                alvr_gui_common::tooltip(
                    ui,
                    "real_time_tooltip",
                    "This setting can be changed in real-time during streaming!",
                );
            }
        });

        if let Some(string) = &self.notice {
            notice::notice(ui, string);

            ui.end_row();

            ui.label(" ");
        }

        response = self
            .control
            .ui(ui, &mut self.preset_json, true)
            .or(response);

        if let Some(desc) = response {
            // todo: handle children requests
            self.modifiers[desc.value.as_str().unwrap()].clone()
        } else {
            vec![]
        }
    }
}
