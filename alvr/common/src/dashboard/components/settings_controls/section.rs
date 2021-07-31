use super::{
    EmptyContainer, EmptyControl, SettingContainer, SettingControl, SettingsContext,
    SettingsResponse,
};
use egui::{Grid, Ui};
use serde_json as json;
use settings_schema::EntryData;
use std::{collections::HashMap, sync::atomic::AtomicUsize};

fn get_id() -> usize {
    lazy_static::lazy_static! {
        static ref COUNTER: AtomicUsize = AtomicUsize::new(0);
    }

    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

enum DisplayMode {
    OnlyBasic,
    OnlyAdvanced,
    Always,
}

struct Entry {
    display_mode: DisplayMode,
    name: String,
    display_name: String,
    help: Option<String>,
    notice: Option<String>,
    control: Box<dyn SettingControl>,
    container: Box<dyn SettingContainer>,
}

pub struct Section {
    id: usize, // the id is used to disambiguate grid containers and avoid disappearing entries
    entries: Vec<Entry>,
}

impl Section {
    pub fn new(entries: Vec<(String, Option<EntryData>)>, session_fragment: json::Value) -> Self {
        let mut session_entries =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            id: get_id(),
            entries: entries
                .into_iter()
                .map(|(name, data)| {
                    let display_name = name.clone();

                    if let Some(data) = data {
                        let session_entry = session_entries.remove(&name).unwrap();

                        Entry {
                            display_mode: if data.advanced {
                                DisplayMode::OnlyAdvanced
                            } else {
                                DisplayMode::Always
                            },
                            name,
                            display_name,
                            help: None,
                            notice: None,
                            control: super::create_setting_control(
                                data.content.clone(),
                                session_entry.clone(),
                            ),
                            container: super::create_setting_container(data.content, session_entry),
                        }
                    } else {
                        // todo
                        Entry {
                            display_mode: DisplayMode::OnlyBasic,
                            name,
                            display_name,
                            help: None,
                            notice: None,
                            control: Box::new(EmptyControl),
                            container: Box::new(EmptyContainer),
                        }
                    }
                })
                .collect(),
        }
    }
}

impl Section {
    pub fn ui_no_indentation(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let session_entries =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Grid::new(format!("section_entries{}", self.id))
            .striped(true)
            .min_col_width(ui.available_width())
            .show(ui, |ui| {
                let mut response = None;
                for entry in &mut self.entries {
                    let session_entry = session_entries
                        .get(&entry.name)
                        .cloned()
                        .unwrap_or(json::Value::Null);

                    if (context.advanced && !matches!(entry.display_mode, DisplayMode::OnlyBasic))
                        || (!context.advanced
                            && !matches!(entry.display_mode, DisplayMode::OnlyAdvanced))
                    {
                        let entry_response = ui
                            .vertical(|ui| {
                                let response = ui
                                    .horizontal({
                                        let entry_session = session_entry.clone();
                                        |ui| {
                                            let res = ui.label(&entry.display_name);
                                            if let Some(help) = &entry.help {
                                                res.on_hover_text(help);
                                            }
                                            entry.control.ui(ui, entry_session, context)
                                        }
                                    })
                                    .inner;

                                if let Some(notice) = &entry.notice {
                                    ui.group(|ui| ui.label(notice));
                                }

                                entry
                                    .container
                                    .ui(ui, session_entry.clone(), context)
                                    .or(response)
                            })
                            .inner;

                        ui.end_row();

                        response = response.or_else({
                            let mut session_entries = session_entries.clone();
                            move || {
                                super::map_fragment(entry_response, |res| {
                                    session_entries.insert(entry.name.clone(), res);
                                    session_entries
                                })
                            }
                        });
                    }
                }

                response
            })
            .inner
    }
}

impl SettingContainer for Section {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        // adds indentation
        super::container(ui, |ui| {
            self.ui_no_indentation(ui, session_fragment, context)
        })

        // todo: no not render container if all entries are hidden (because advanced)
    }
}
