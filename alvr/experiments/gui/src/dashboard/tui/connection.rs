use alvr_session::SessionDesc;
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

pub struct ConnectionPanel {
    selected: usize,
}

impl ConnectionPanel {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, area: Rect, session: &SessionDesc) {
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(new_clients.len() as u16 + 2),
                Constraint::Length(trusted_clients.len() as u16 + 2),
                Constraint::Min(0),
            ])
            .split(area);

        let new_clients_block = Block::default().borders(Borders::ALL).title("New clients");
        frame.render_widget(new_clients_block, chunks[0]);

        let trusted_clients_block = Block::default()
            .borders(Borders::ALL)
            .title("Trusted clients");
        frame.render_widget(trusted_clients_block, chunks[1]);
    }

    pub fn react_to_key(&self, key: Key) {}
}
