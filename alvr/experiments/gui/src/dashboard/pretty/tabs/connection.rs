use std::net::IpAddr;

use alvr_session::SessionDesc;
use iced::{
    alignment::Horizontal, button, container, scrollable, Alignment, Button, Container, Element,
    Length, Row, Scrollable, Space, Text,
};

use crate::dashboard::{
    pretty::theme::{ButtonStyle, ScrollableStyle, ELEMENT_BACKGROUND},
    RequestHandler,
};

#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    SessionUpdated(SessionDesc),
    AddClient(String, IpAddr),
    TrustClient(String),
    RemoveClient(String),
}

struct ClientEntry {
    display_name: String,
    hostname: String,
    button_state: button::State,
}

pub struct ConnectionPanel {
    new_clients: Vec<ClientEntry>,
    trusted_clients: Vec<ClientEntry>,
    scrollable_state: scrollable::State,
}

impl ConnectionPanel {
    pub fn new() -> Self {
        Self {
            new_clients: vec![],
            trusted_clients: vec![],
            scrollable_state: scrollable::State::new(),
        }
    }

    pub fn update(&mut self, event: ConnectionEvent, request_handler: &mut RequestHandler) {
        match event {
            ConnectionEvent::SessionUpdated(session) => {
                self.new_clients = session
                    .client_connections
                    .iter()
                    .filter_map(|(hostname, conn_desc)| {
                        (!conn_desc.trusted).then(|| ClientEntry {
                            display_name: conn_desc.display_name.clone(),
                            hostname: hostname.clone(),
                            button_state: Default::default(),
                        })
                    })
                    .collect::<Vec<_>>();
                self.trusted_clients = session
                    .client_connections
                    .iter()
                    .filter_map(|(hostname, conn_desc)| {
                        conn_desc.trusted.then(|| ClientEntry {
                            display_name: conn_desc.display_name.clone(),
                            hostname: hostname.clone(),
                            button_state: Default::default(),
                        })
                    })
                    .collect::<Vec<_>>();
            }
            ConnectionEvent::AddClient(hostname, ip_address) => {
                request_handler(format!(r#"add_client("{}", "{}")"#, hostname, ip_address));
            }
            ConnectionEvent::TrustClient(hostname) => {
                request_handler(format!(r#"trust_client("{}")"#, hostname));
            }
            ConnectionEvent::RemoveClient(hostname) => {
                request_handler(format!(r#"remove_client("{}")"#, hostname));
            }
        }
    }

    pub fn view(&mut self) -> Element<ConnectionEvent> {
        let mut scrollable = Scrollable::new(&mut self.scrollable_state)
            .style(ScrollableStyle)
            .spacing(10)
            .padding(10);

        scrollable = scrollable.push(Text::new("New clients").size(20));

        if !self.new_clients.is_empty() {
            for client in &mut self.new_clients {
                scrollable = scrollable.push(
                    Container::new(
                        Row::with_children(vec![
                            Text::new(client.display_name.clone()).size(17).into(),
                            Space::with_width(Length::Fill).into(),
                            Text::new(format!("hostname: {}", client.hostname)).into(),
                            Space::with_width(Length::Fill).into(),
                            Button::new(&mut client.button_state, Text::new("Trust"))
                                .style(ButtonStyle::Primary)
                                .on_press(ConnectionEvent::TrustClient(client.hostname.clone()))
                                .into(),
                        ])
                        .align_items(Alignment::Center)
                        .padding(10),
                    )
                    .style(ClientCardStyle),
                )
            }
        } else {
            scrollable =
                scrollable.push(Text::new("No clients").horizontal_alignment(Horizontal::Center))
        }

        scrollable = scrollable.push(Space::with_height(10.into()));

        scrollable = scrollable.push(Text::new("Trusted clients").size(20));

        if !self.trusted_clients.is_empty() {
            for client in &mut self.trusted_clients {
                scrollable = scrollable.push(
                    Container::new(
                        Row::with_children(vec![
                            Text::new(client.display_name.clone()).size(17).into(),
                            Space::with_width(Length::Fill).into(),
                            Text::new(format!("hostname: {}", client.hostname)).into(),
                            Space::with_width(Length::Fill).into(),
                            Button::new(&mut client.button_state, Text::new("Remove"))
                                .style(ButtonStyle::Danger)
                                .on_press(ConnectionEvent::RemoveClient(client.hostname.clone()))
                                .into(),
                        ])
                        .align_items(Alignment::Center)
                        .padding(10),
                    )
                    .style(ClientCardStyle),
                )
            }
        } else {
            scrollable =
                scrollable.push(Text::new("No clients").horizontal_alignment(Horizontal::Center))
        }

        scrollable.into()
    }
}

struct ClientCardStyle;

impl container::StyleSheet for ClientCardStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: ELEMENT_BACKGROUND.into(),
            border_radius: 10.0,
            ..Default::default()
        }
    }
}
