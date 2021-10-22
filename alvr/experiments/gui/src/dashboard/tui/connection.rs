use alvr_session::{ClientConnectionDesc, SessionDesc};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::dashboard::{ClientListAction, ConnectionsEvent, DashboardEvent};

#[allow(clippy::type_complexity)]
fn split_clients(
    session: &SessionDesc,
) -> (
    Vec<(&String, &ClientConnectionDesc)>,
    Vec<(&String, &ClientConnectionDesc)>,
) {
    let new_clients = session
        .client_connections
        .iter()
        .filter_map(|(hostname, conn_desc)| (!conn_desc.trusted).then(|| (hostname, conn_desc)))
        .collect::<Vec<_>>();
    let trusted_clients = session
        .client_connections
        .iter()
        .filter_map(|(hostname, conn_desc)| conn_desc.trusted.then(|| (hostname, conn_desc)))
        .collect::<Vec<_>>();

    (new_clients, trusted_clients)
}

pub struct ConnectionPanel {
    list_state: ListState,
}

impl ConnectionPanel {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect, session: &SessionDesc) {
        let (new_clients, trusted_clients) = split_clients(session);
        let new_clients_len = new_clients.len();
        let trusted_clients_len = trusted_clients.len();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(new_clients_len as u16 + 2),
                Constraint::Length(trusted_clients_len as u16 + 2),
                Constraint::Min(0),
            ])
            .split(area);

        let new_clients_list = List::new(
            new_clients
                .into_iter()
                .map(|(hostname, conn_desc)| {
                    ListItem::new(format!(
                        "{}  hostname: {}",
                        conn_desc.display_name, hostname
                    ))
                })
                .collect::<Vec<_>>(),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("Trust>  ")
        .block(Block::default().borders(Borders::ALL).title("New clients"));

        frame.render_stateful_widget(new_clients_list, chunks[0], &mut self.list_state);

        let trusted_clients_list = List::new(
            trusted_clients
                .into_iter()
                .map(|(hostname, conn_desc)| {
                    ListItem::new(format!("{} hostname: {}", conn_desc.display_name, hostname))
                })
                .collect::<Vec<_>>(),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("Remove> ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Trusted clients"),
        );

        let selection = self.list_state.selected().unwrap();
        let mut state2 = ListState::default();
        state2.select(Some(if selection < new_clients_len {
            usize::MAX
        } else {
            selection - new_clients_len
        }));
        frame.render_stateful_widget(trusted_clients_list, chunks[1], &mut state2);
    }

    pub fn react_to_key(
        &mut self,
        key: Key,
        session: &SessionDesc,
        event_handler: &mut impl FnMut(DashboardEvent),
    ) {
        match key {
            Key::Up => {
                let selection = self.list_state.selected().unwrap();
                if selection > 0 {
                    self.list_state.select(Some(selection - 1));
                }
            }
            Key::Down => {
                let selection = self.list_state.selected().unwrap();
                if selection < session.client_connections.len() - 1 {
                    self.list_state.select(Some(selection + 1));
                }
            }
            Key::Char('\n') => {
                let selection = self.list_state.selected().unwrap();
                let (new_clients, trusted_clients) = split_clients(session);

                if selection < new_clients.len() {
                    let (hostname, _) = new_clients[selection];
                    event_handler(DashboardEvent::Connections(ConnectionsEvent {
                        hostname: hostname.clone(),
                        action: ClientListAction::TrustAndMaybeAddIp(None),
                    }));
                } else if selection < session.client_connections.len() {
                    let (hostname, _) = trusted_clients[selection - new_clients.len()];
                    event_handler(DashboardEvent::Connections(ConnectionsEvent {
                        hostname: hostname.clone(),
                        action: ClientListAction::RemoveIpOrEntry(None),
                    }));
                } // else something is wrong, skip
            }
            _ => (),
        }
    }
}
