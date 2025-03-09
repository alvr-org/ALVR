use super::{collapsible, notice, NestingInfo, SettingControl, INDENTATION_STEP};
use alvr_gui_common::{
    theme::{
        log_colors::{INFO_LIGHT, WARNING_LIGHT},
        OK_GREEN,
    },
    DisplayString,
};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::{SchemaEntry, SchemaNode};
use eframe::egui::Ui;
use serde_json as json;

struct Entry {
    id: DisplayString,
    help: Option<String>,
    notice: Option<String>,
    hidden: bool,
    steamvr_restart_flag: bool,
    real_time_flag: bool,
    control: SettingControl,
}

pub struct Control {
    nesting_info: NestingInfo,
    entries: Vec<Entry>,
    gui_collapsible: bool,
}

impl Control {
    pub fn new(
        mut nesting_info: NestingInfo,
        schema_entries: Vec<SchemaEntry<SchemaNode>>,
        gui_collapsible: bool,
    ) -> Self {
        nesting_info.indentation_level += 1;

        let entries = schema_entries
            .into_iter()
            .map(|entry| {
                let id = entry.name;
                let display = super::get_display_name(&id, &entry.strings);
                let help = entry.strings.get("help").cloned();
                let notice = entry.strings.get("notice").cloned();
                let hidden = entry.flags.contains("hidden");
                let steamvr_restart_flag = entry.flags.contains("steamvr-restart");
                let real_time_flag = entry.flags.contains("real-time");

                let mut nesting_info = nesting_info.clone();
                nesting_info.path.push(id.clone().into());

                Entry {
                    id: DisplayString { id, display },
                    help,
                    notice,
                    hidden,
                    steamvr_restart_flag,
                    real_time_flag,
                    control: SettingControl::new(nesting_info, entry.content),
                }
            })
            .collect();

        Self {
            nesting_info,
            entries,
            gui_collapsible,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        let entries_count = self.entries.len();

        let mut request = None;

        let collapsed = if self.gui_collapsible {
            super::grid_flow_inline(ui, allow_inline);

            let collapsed = collapsible::collapsible_button(
                ui,
                &self.nesting_info,
                session_fragment,
                &mut request,
            );

            if !collapsed {
                ui.end_row();
            }

            collapsed
        } else {
            if allow_inline {
                ui.end_row();
            }

            false
        };

        if !collapsed {
            for (i, entry) in self.entries.iter_mut().enumerate() {
                if entry.hidden {
                    continue;
                }

                ui.horizontal(|ui| {
                    ui.add_space(INDENTATION_STEP * self.nesting_info.indentation_level as f32);
                    let label_res = ui.label(&entry.id.display);
                    if cfg!(debug_assertions) {
                        label_res.on_hover_text(&*entry.id);
                    }

                    if let Some(string) = &entry.help {
                        if ui.colored_label(INFO_LIGHT, "❓").hovered() {
                            alvr_gui_common::tooltip(
                                ui,
                                &format!("{}_help_tooltip", entry.id.display),
                                string,
                            );
                        }
                    }
                    if entry.steamvr_restart_flag && ui.colored_label(WARNING_LIGHT, "⚠").hovered()
                    {
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
                    if entry.real_time_flag && ui.colored_label(OK_GREEN, "🔵").hovered() {
                        alvr_gui_common::tooltip(
                            ui,
                            "real_time_tooltip",
                            "This setting can be changed in real-time during streaming!",
                        );
                    }
                });

                if let Some(string) = &entry.notice {
                    notice::notice(ui, string);

                    ui.end_row();

                    ui.label(" ");
                }

                request = entry
                    .control
                    .ui(ui, &mut session_fragment[&entry.id.id], true)
                    .or(request);

                if i != entries_count - 1 {
                    ui.end_row();
                }
            }
        }

        request
    }
}
