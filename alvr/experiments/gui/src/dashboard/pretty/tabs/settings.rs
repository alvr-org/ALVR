use std::collections::HashMap;

use crate::dashboard::{pretty::theme::ButtonStyle, RequestHandler};

use super::{settings_controls::SectionControl, SettingControl, SettingEvent};
use alvr_session::SessionDesc;
use iced::{scrollable, Button, Column, Element, Length, Row, Scrollable, Space, Text};
use iced_native::button;
use serde_json as json;
use settings_schema::SchemaNode;

#[derive(Clone, Debug)]
pub enum SettingsPanelEvent {
    SessionUpdated(SessionDesc),
    TabClick(usize),
    AdvancedClick,
    Inner(SettingEvent), // the tab is known
}

pub struct TabLabelState {
    name: String,
    display_name: String,
    label_state: button::State,
}

pub struct TabContentState {
    name: String,
    content: SettingControl,
    scroll_state: scrollable::State,
}

pub struct SettingsPanel {
    // labels and content is split to satisfy lifetimes in view()
    tabs_labels: Vec<TabLabelState>,
    tabs_content: Vec<TabContentState>,
    selected_tab: usize,
    advanced: bool,
    advanced_button_state: button::State,
}

impl SettingsPanel {
    pub fn new(session: &SessionDesc, request_handler: &mut RequestHandler) -> Self {
        let schema = alvr_session::settings_schema(alvr_session::session_settings_default());
        let session_tabs = json::from_value::<HashMap<String, json::Value>>(
            json::to_value(&session.session_settings).unwrap(),
        )
        .unwrap();

        let (tabs_labels, tabs_content);
        if let SchemaNode::Section { entries } = schema {
            tabs_labels = entries
                .iter()
                .map(|(name, maybe_data)| {
                    if let Some(data) = maybe_data {
                        TabLabelState {
                            name: name.clone(),
                            display_name: name.clone(),
                            label_state: button::State::new(),
                        }
                    } else {
                        unreachable!()
                    }
                })
                .collect();
            tabs_content = entries
                .into_iter()
                .map(|(name, maybe_data)| {
                    if let Some(data) = maybe_data {
                        TabContentState {
                            name: name.clone(),
                            content: SettingControl::new(
                                format!("session.sessionSettings.{}", name),
                                data.content,
                                session_tabs.get(&name).unwrap().clone(),
                                request_handler,
                            ),
                            scroll_state: scrollable::State::new(),
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
            advanced: session.advanced,
            advanced_button_state: button::State::new(),
        }
    }

    pub fn update(&mut self, event: SettingsPanelEvent, request_handler: &mut RequestHandler) {
        match event {
            SettingsPanelEvent::SessionUpdated(session) => {
                self.advanced = session.advanced;
                let session_tabs = json::from_value::<HashMap<String, json::Value>>(
                    json::to_value(session.session_settings).unwrap(),
                )
                .unwrap();
                for tab in &mut self.tabs_content {
                    tab.content.update(
                        SettingEvent::SettingsUpdated(session_tabs[&tab.name].clone()),
                        request_handler,
                    );
                }
            }
            SettingsPanelEvent::TabClick(tab_index) => self.selected_tab = tab_index,
            SettingsPanelEvent::AdvancedClick => {
                request_handler(format!(
                    r#"
                        let session = load_session();
                        session.advanced = {};
                        store_session(session);
                    "#,
                    !self.advanced
                ));
            }
            SettingsPanelEvent::Inner(event) => self.tabs_content[self.selected_tab]
                .content
                .update(event, request_handler),
        }
    }

    pub fn view(&mut self) -> Element<SettingsPanelEvent> {
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
                                .on_press(SettingsPanelEvent::TabClick(index))
                                .into()
                        })
                        .collect(),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    Button::new(&mut self.advanced_button_state, Text::new("Advanced")).style(
                        if self.advanced {
                            ButtonStyle::Primary
                        } else {
                            ButtonStyle::Secondary
                        },
                    ),
                ),
            )
            .push({
                let active_tab = &mut self.tabs_content[self.selected_tab];

                // let label_elements = active_tab.content.label_elements(self.advanced);
                // let control_elements = active_tab.content.control_elements(self.advanced);

                Scrollable::new(&mut active_tab.scroll_state) // .push(Column::new().push(label_elements))
                                                              // .push(Column::new().push(control_elements))
            })
            .into()
    }
}
