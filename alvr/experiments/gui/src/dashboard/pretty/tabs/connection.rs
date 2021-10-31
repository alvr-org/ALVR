use std::net::IpAddr;

use alvr_session::SessionDesc;
use iced::{
    alignment::Horizontal, button, container, scrollable, Alignment, Button, Container, Element,
    Length, Row, Scrollable, Space, Text,
};

use crate::dashboard::{
    pretty::theme::{ButtonStyle, ScrollableStyle, BACKGROUND_SECONDARY},
    RequestHandler,
};

#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    SessionUpdated(SessionDesc),
    AddClient(String, IpAddr),
    TrustClient(String),
    RemoveClient(String),
}

struct ClientState {
    display_name: String,
    hostname: String,
    button_state: button::State,
}

struct SplitClients {
    new: Vec<ClientState>,
    trusted: Vec<ClientState>,
}

fn split_clients(session: &SessionDesc) -> SplitClients {
    let new = session
        .client_connections
        .iter()
        .filter_map(|(hostname, conn_desc)| {
            (!conn_desc.trusted).then(|| ClientState {
                display_name: conn_desc.display_name.clone(),
                hostname: hostname.clone(),
                button_state: Default::default(),
            })
        })
        .collect::<Vec<_>>();
    let trusted = session
        .client_connections
        .iter()
        .filter_map(|(hostname, conn_desc)| {
            conn_desc.trusted.then(|| ClientState {
                display_name: conn_desc.display_name.clone(),
                hostname: hostname.clone(),
                button_state: Default::default(),
            })
        })
        .collect::<Vec<_>>();

    SplitClients { new, trusted }
}

pub struct ConnectionPanel {
    clients: SplitClients,
    scrollable_state: scrollable::State,
}

impl ConnectionPanel {
    pub fn new(session: &SessionDesc) -> Self {
        Self {
            clients: split_clients(session),
            scrollable_state: Default::default(),
        }
    }

    pub fn update(&mut self, event: ConnectionEvent, request_handler: &mut RequestHandler) {
        match event {
            ConnectionEvent::SessionUpdated(session) => self.clients = split_clients(&session),
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

        if !self.clients.new.is_empty() {
            for client in &mut self.clients.new {
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

        if !self.clients.trusted.is_empty() {
            for client in &mut self.clients.trusted {
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
            background: BACKGROUND_SECONDARY.into(),
            border_radius: 10.0,
            ..Default::default()
        }
    }
}
