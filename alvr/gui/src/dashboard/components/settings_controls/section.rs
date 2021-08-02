use crate::{translation::TranslationBundle, LocalizedId};

use super::{
    EmptyContainer, EmptyControl, SettingContainer, SettingControl, SettingsContext,
    SettingsResponse,
};
use egui::{Grid, Ui};
use serde_json as json;
use settings_schema::EntryData;
use std::{collections::HashMap, sync::atomic::AtomicUsize};

const CONTROLS_TARGET_OFFSET: f32 = 200_f32;

fn get_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn entry_trans(trans: &TranslationBundle, trans_path_parent: &str, id: &str) -> String {
    trans
        .fallible_with_args(&format!("{}-{}", trans_path_parent, id), None)
        .unwrap_or_else(|| id.to_owned())
}

enum DisplayMode {
    OnlyBasic,
    OnlyAdvanced,
    Always,
}

struct Entry {
    display_mode: DisplayMode,
    id: LocalizedId,
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
    pub fn new(
        entries: Vec<(String, Option<EntryData>)>,
        session_fragment: json::Value,
        trans_path: &str,
        trans: &TranslationBundle,
    ) -> Self {
        let mut session_entries =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            id: get_id(),
            entries: entries
                .into_iter()
                .map(|(id, data)| {
                    let id = LocalizedId {
                        id: id.clone(),
                        trans: entry_trans(trans, trans_path, &id),
                    };
                    let entry_trans_path = format!("{}-{}", trans_path, *id);

                    if let Some(data) = data {
                        let session_entry = session_entries.remove(&*id).unwrap();

                        Entry {
                            display_mode: if data.advanced {
                                DisplayMode::OnlyAdvanced
                            } else {
                                DisplayMode::Always
                            },
                            id,
                            help: trans.attribute_fallible_with_args(
                                &entry_trans_path,
                                "help",
                                None,
                            ),
                            notice: trans.attribute_fallible_with_args(
                                &entry_trans_path,
                                "notice",
                                None,
                            ),
                            control: super::create_setting_control(
                                data.content.clone(),
                                session_entry.clone(),
                                &entry_trans_path,
                                trans,
                            ),
                            container: super::create_setting_container(
                                data.content,
                                session_entry,
                                &entry_trans_path,
                                trans,
                            ),
                        }
                    } else {
                        // todo
                        Entry {
                            display_mode: DisplayMode::OnlyBasic,
                            id,
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
            .max_col_width(ui.available_width())
            .show(ui, |ui| {
                let mut response = None;
                for entry in &mut self.entries {
                    let session_entry = session_entries
                        .get(&*entry.id)
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
                                            let res = ui.label(&entry.id.trans);
                                            if let Some(help) = &entry.help {
                                                res.on_hover_text(help);
                                            }

                                            // Align controls
                                            let left_offset =
                                                context.view_width - ui.available_width();
                                            if left_offset < CONTROLS_TARGET_OFFSET {
                                                ui.add_space(CONTROLS_TARGET_OFFSET - left_offset);
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
                                    session_entries.insert((*entry.id).clone(), res);
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
