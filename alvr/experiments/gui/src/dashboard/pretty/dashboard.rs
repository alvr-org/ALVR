use super::{
    tabs::{ConnectionEvent, ConnectionPanel, SettingsEvent, SettingsPanel},
    theme::{ContainerStyle, ACCENT, ELEMENT_BACKGROUND, FOREGROUND},
};
use crate::dashboard::{pretty::theme::ContainerSecondaryStyle, RequestHandler};
use alvr_session::ServerEvent;
use iced::{
    alignment::Horizontal, button, Alignment, Button, Column, Container, Element, Length, Row,
    Space, Text,
};

pub enum TabLabelStyle {
    Normal,
    Selected,
}

impl button::StyleSheet for TabLabelStyle {
    fn active(&self) -> button::Style {
        let normal = button::Style {
            background: ELEMENT_BACKGROUND.into(),
            border_radius: 10.0,
            text_color: FOREGROUND,
            ..Default::default()
        };

        match self {
            TabLabelStyle::Normal => normal,
            TabLabelStyle::Selected => button::Style {
                background: ACCENT.into(),
                ..normal
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum DashboardEvent {
    ServerEvent(ServerEvent),
    TabClick(usize),
    LanguageClick,
    ConnectionEvent(ConnectionEvent),
    SettingsEvent(SettingsEvent),
}

pub struct TabState {
    icon: (), // todo
    display_name: String,
    state: button::State,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            icon: (),
            display_name: "".into(),
            state: Default::default(),
        }
    }
}

pub struct Dashboard {
    selected_tab: usize,
    tab_states: Vec<TabState>,
    language_state: TabState,
    connection_panel: ConnectionPanel,
    settings_panel: SettingsPanel,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            tab_states: vec![
                TabState {
                    display_name: "Connection".into(),
                    ..Default::default()
                },
                TabState {
                    display_name: "Statistics".into(),
                    ..Default::default()
                },
                TabState {
                    display_name: "Settings".into(),
                    ..Default::default()
                },
                TabState {
                    display_name: "Installation".into(),
                    ..Default::default()
                },
                TabState {
                    display_name: "Logs".into(),
                    ..Default::default()
                },
                TabState {
                    display_name: "About".into(),
                    ..Default::default()
                },
            ],
            language_state: TabState {
                display_name: "Language".into(),
                ..Default::default()
            },
            connection_panel: ConnectionPanel::new(),
            settings_panel: SettingsPanel::new(),
        }
    }

    pub fn update(&mut self, event: DashboardEvent, request_handler: &mut RequestHandler) {
        match event {
            DashboardEvent::ServerEvent(event) => match event {
                ServerEvent::Session(session) => {
                    self.connection_panel.update(
                        ConnectionEvent::SessionUpdated((*session).clone()),
                        request_handler,
                    );
                    self.settings_panel
                        .update(SettingsEvent::SessionUpdated(*session), request_handler);
                }
                ServerEvent::SessionUpdated => (), // deprecated
                ServerEvent::SessionSettingsExtrapolationFailed => todo!(),
                ServerEvent::ClientFoundOk => todo!(),
                ServerEvent::ClientFoundInvalid => todo!(),
                ServerEvent::ClientFoundWrongVersion(_) => todo!(),
                ServerEvent::ClientConnected => todo!(),
                ServerEvent::ClientDisconnected => todo!(),
                ServerEvent::UpdateDownloadedBytesCount(_) => todo!(),
                ServerEvent::UpdateDownloadError => todo!(),
                ServerEvent::Statistics(_) => todo!(),
                ServerEvent::ServerQuitting => unreachable!(),
                ServerEvent::Raw(_) => (),
                ServerEvent::EchoQuery(_) => todo!(),
            },
            DashboardEvent::TabClick(tab) => self.selected_tab = tab,
            DashboardEvent::LanguageClick => (),
            DashboardEvent::ConnectionEvent(event) => {
                self.connection_panel.update(event, request_handler)
            }
            DashboardEvent::SettingsEvent(event) => {
                self.settings_panel.update(event, request_handler)
            }
        }
    }

    pub fn view(&mut self) -> Element<DashboardEvent> {
        let mut tab_labels = vec![Text::new("ALVR")
            .size(20)
            .horizontal_alignment(Horizontal::Center)
            .into()];

        // work around "self.tab_states cannot be borrowed both mutably and immutably"
        let mut selected_tab_display_name = "".into();

        for (index, state) in self.tab_states.iter_mut().enumerate() {
            if index == self.selected_tab {
                selected_tab_display_name = state.display_name.clone();
            }

            tab_labels.push(
                Button::new(
                    &mut state.state,
                    Row::new()
                        // .push(Image::new(image::Handle::from_memory(
                        //     include_bytes!("../../../resources/images/favicon.png").to_vec(),
                        // )))
                        .push(Text::new(&state.display_name))
                        .spacing(10),
                )
                .style(if self.selected_tab == index {
                    TabLabelStyle::Selected
                } else {
                    TabLabelStyle::Normal
                })
                .padding([7, 10])
                .on_press(DashboardEvent::TabClick(index))
                .into(),
            );
        }

        let sidebar = Container::new(
            Column::with_children(tab_labels)
                .push(Space::with_height(Length::Fill))
                .push(
                    Button::new(
                        &mut self.language_state.state,
                        Text::new(&self.language_state.display_name),
                    )
                    .style(TabLabelStyle::Normal)
                    .padding([7, 10])
                    .on_press(DashboardEvent::LanguageClick),
                )
                .padding(10)
                .spacing(10)
                .align_items(Alignment::Fill),
        )
        .style(ContainerSecondaryStyle);

        let content_panel = match self.selected_tab {
            0 => self
                .connection_panel
                .view()
                .map(DashboardEvent::ConnectionEvent),
            2 => self
                .settings_panel
                .view()
                .map(DashboardEvent::SettingsEvent),
            _ => Text::new("unimplemented").into(),
        };

        Container::new(
            Row::new().push(sidebar).push(
                Column::new()
                    .push(
                        Container::new(Text::new(selected_tab_display_name).size(30))
                            .padding([10, 20]),
                    )
                    .push(content_panel)
                    .width(Length::Fill),
            ),
        )
        .style(ContainerStyle)
        .into()
    }
}
