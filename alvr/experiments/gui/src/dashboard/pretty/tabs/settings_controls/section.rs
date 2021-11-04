use super::{DrawingData, InitData, SettingControl, ShowMode, UpdatingData};
use crate::dashboard::pretty::tabs::{higher_order, SettingControlEventType};
use iced::{button, Element, Text};
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

struct HelpButtonState {
    state: button::State,
    text: String,
}

struct Entry {
    show_mode: ShowMode,
    name: String,
    display_name: String,
    control: SettingControl,
    help_button_state: Option<HelpButtonState>,
    notice: Option<String>,
}

pub struct Control {
    entries: Vec<Entry>,
}

impl Control {
    pub fn new(data: InitData<Vec<(String, Option<EntryData>)>>) -> Self {
        let entries = data
            .schema
            .into_iter()
            .map(|(name, maybe_data)| {
                if let Some(data) = maybe_data {
                    let show_mode = if data.advanced {
                        ShowMode::Advanced
                    } else {
                        ShowMode::Always
                    };
                    let control = SettingControl::new(InitData {
                        schema: data.content,
                        trans: (),
                    });

                    Entry {
                        show_mode,
                        name: name.clone(),
                        display_name: name, // todo
                        control,
                        help_button_state: None, // todo
                        notice: None,            // todo
                    }
                } else {
                    // todo
                    let control = SettingControl::HigherOrder(higher_order::Control::new(()));

                    Entry {
                        show_mode: ShowMode::Basic,
                        name: name.clone(),
                        display_name: name,
                        control,
                        help_button_state: None,
                        notice: None,
                    }
                }
            })
            .collect();

        Self { entries }
    }

    pub fn update(&mut self, mut data: UpdatingData) {
        match data.event {
            SettingControlEventType::SessionUpdated(session) => {
                let session_entries =
                    json::from_value::<HashMap<String, json::Value>>(session).unwrap();

                for entry in &mut self.entries {
                    let session = session_entries
                        .get(&entry.name)
                        .cloned()
                        .unwrap_or(json::Value::Null); // in case of HOS or custom controls
                    entry.control.update(UpdatingData {
                        path: vec![],
                        event: SettingControlEventType::SessionUpdated(session),
                        request_handler: data.request_handler,
                        string_path: String::new(),
                    })
                }
            }
            _ => {
                let entry = &mut self.entries[data.path.pop().unwrap()];
                let string_path = format!("{}.{}", data.string_path, entry.name);
                entry.control.update(UpdatingData {
                    path: data.path,
                    event: data.event,
                    request_handler: data.request_handler,
                    string_path,
                })
            }
        }
    }
}
