use super::{
    DrawingData, DrawingResult, InitData, SettingControl, SettingControlEvent, ShowMode,
    UpdatingData, INDENTATION,
};
use crate::dashboard::pretty::tabs::{
    higher_order, settings_controls::ROW_HEIGHT, SettingControlEventType,
};
use iced::{button, Column, Element, Length, Row, Space, Text};
use iced_native::Widget;
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
        if let SettingControlEventType::SessionUpdated(session) = data.event {
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
        } else {
            let entry = &mut self.entries[data.path.pop().unwrap()];
            entry.control.update(UpdatingData {
                string_path: format!("{}.{}", data.string_path, entry.name),
                ..data
            })
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let (left_controls, right_controls): (Vec<_>, Vec<_>) = self
            .entries
            .iter_mut()
            .enumerate()
            .filter_map(|(index, entry)| {
                (entry.show_mode == ShowMode::Always
                    || (entry.show_mode == ShowMode::Advanced && data.advanced)
                    || (entry.show_mode == ShowMode::Basic && !data.advanced))
                    .then(|| {
                        let result = entry.control.view(data);
                        let mut left_control = Column::new()
                            .push(Text::new(entry.display_name.clone()).height(ROW_HEIGHT));
                        let mut right_control = Column::new().push(
                            result
                                .inline
                                .unwrap_or_else(|| Space::with_height(ROW_HEIGHT).into()),
                        );

                        if let Some(notice) = &entry.notice {
                            todo!() // Add space on left control, notice card on right control
                        }

                        let left_control: Element<_> = left_control
                            .push(
                                Row::new()
                                    .push(Space::with_width(INDENTATION))
                                    .push(result.left),
                            )
                            .into();
                        let right_control: Element<_> = right_control.push(result.right).into();

                        (
                            left_control.map(move |mut e: SettingControlEvent| {
                                e.path.push(index);
                                e
                            }),
                            right_control.map(move |mut e: SettingControlEvent| {
                                e.path.push(index);
                                e
                            }),
                        )
                    })
            })
            .unzip();

        let left = Column::with_children(left_controls).into();
        let right = Column::with_children(right_controls).into();

        DrawingResult {
            inline: None,
            left,
            right,
        }
    }
}
