use std::collections::HashMap;

use crate::dashboard::{
    pretty::{
        tabs::InitData,
        theme::{ButtonStyle, ScrollableStyle},
    },
    RequestHandler,
};

use super::{
    DrawingData, DrawingResult, SettingControl, SettingControlEvent, SettingControlEventType,
    UpdatingData,
};
use alvr_session::SessionDesc;
use iced::{scrollable, Button, Column, Element, Length, Row, Scrollable, Space, Text};
use iced_native::button;
use serde_json as json;
use settings_schema::SchemaNode;

#[derive(Clone, Debug)]
pub enum SettingsEvent {
    SessionUpdated(SessionDesc),
    TabClick(usize),
    AdvancedClick,
    FromControl(SettingControlEvent),
}

pub struct TabLabel {
    name: String,
    display_name: String,
    label_state: button::State,
}

pub struct TabContent {
    name: String,
    scroll_state: scrollable::State,
    control: SettingControl,
}

pub struct SettingsPanel {
    // labels and content is split to satisfy lifetimes in view()
    tabs_labels: Vec<TabLabel>,
    tabs_content: Vec<TabContent>,
    selected_tab: usize,
    advanced: bool,
    advanced_button_state: button::State,
}

impl SettingsPanel {
    pub fn new(request_handler: &mut RequestHandler) -> Self {
        let schema = alvr_session::settings_schema(alvr_session::session_settings_default());
        let (tabs_labels, tabs_content);
        if let SchemaNode::Section { entries } = schema {
            tabs_labels = entries
                .iter()
                .map(|(name, maybe_data)| TabLabel {
                    name: name.clone(),
                    display_name: name.clone(),
                    label_state: button::State::new(),
                })
                .collect();
            tabs_content = entries
                .into_iter()
                .map(|(name, maybe_data)| {
                    if let Some(data) = maybe_data {
                        let control = SettingControl::new(InitData {
                            schema: data.content,
                            trans: (),
                        });

                        TabContent {
                            name,
                            scroll_state: scrollable::State::new(),
                            control,
                        }
                    } else {
                        unreachable!()
                    }
                })
                .collect();
        } else {
            unreachable!();
        };

        Self {
            tabs_labels,
            tabs_content,
            selected_tab: 0,
            advanced: false,
            advanced_button_state: button::State::new(),
        }
    }

    pub fn update(&mut self, event: SettingsEvent, request_handler: &mut RequestHandler) {
        match event {
            SettingsEvent::SessionUpdated(session) => {
                // NB: the SessionUpdated event cannot be just broadcated to every control. Since
                // the session has a tree structure, each descendant is in change of extracting the
                // relevant session portion for their children.

                self.advanced = session.advanced;

                let session_tabs = json::from_value::<HashMap<String, json::Value>>(
                    json::to_value(session.session_settings).unwrap(),
                )
                .unwrap();
                for tab in &mut self.tabs_content {
                    tab.control.update(UpdatingData {
                        path: vec![],
                        event: SettingControlEventType::SessionUpdated(
                            session_tabs[&tab.name].clone(),
                        ),
                        request_handler,
                        string_path: String::new(),
                    });
                }
            }
            SettingsEvent::TabClick(tab_index) => self.selected_tab = tab_index,
            SettingsEvent::AdvancedClick => {
                request_handler(format!(
                    r#"
                        let session = load_session();
                        session.advanced = {};
                        store_session(session);
                    "#,
                    !self.advanced
                ));
            }
            SettingsEvent::FromControl(event) => {
                let tab = &mut self.tabs_content[self.selected_tab];

                tab.control.update(UpdatingData {
                    path: event.path,
                    event: event.event_type,
                    request_handler,
                    string_path: format!("session.session_settings.{}", tab.name),
                })
            }
        }
    }

    pub fn view(&mut self) -> Element<SettingsEvent> {
        Column::new()
            .push(
                Row::with_children(
                    self.tabs_labels
                        .iter_mut()
                        .enumerate()
                        .map(|(index, tab)| {
                            Button::new(&mut tab.label_state, Text::new(&tab.display_name))
                                .style(if index == self.selected_tab {
                                    ButtonStyle::Primary
                                } else {
                                    ButtonStyle::Secondary
                                })
                                .on_press(SettingsEvent::TabClick(index))
                                .into()
                        })
                        .collect(),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    Button::new(&mut self.advanced_button_state, Text::new("Advanced"))
                        .style(if self.advanced {
                            ButtonStyle::Primary
                        } else {
                            ButtonStyle::Secondary
                        })
                        .on_press(SettingsEvent::AdvancedClick),
                )
                .padding(5)
                .spacing(5),
            )
            .push({
                let active_tab = &mut self.tabs_content[self.selected_tab];

                let DrawingResult {
                    inline,
                    left,
                    right,
                } = active_tab.control.view(&DrawingData {
                    advanced: true, //self.advanced,
                    common_trans: (),
                });

                Scrollable::new(&mut active_tab.scroll_state)
                    .push(
                        Row::new()
                            .push(left.map(SettingsEvent::FromControl))
                            .push(right.map(SettingsEvent::FromControl)),
                    )
                    .style(ScrollableStyle)
            })
            .into()
    }
}
