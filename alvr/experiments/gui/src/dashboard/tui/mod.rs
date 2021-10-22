mod about;
mod command;
mod connection;
mod help;
mod installation;
mod logs;
mod notification;

use crate::dashboard::tui::installation::InstallationPanel;

use self::{command::CommandBar, connection::ConnectionPanel, notification::NotificationBar};
use super::DashboardEvent;
use alvr_common::ServerEvent;
use alvr_session::SessionDesc;
use std::{
    cmp,
    collections::VecDeque,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{Block, Borders, Tabs},
    Terminal,
};

pub struct Dashboard {
    session: SessionDesc,
    unprocessed_events: Arc<Mutex<Vec<ServerEvent>>>,

    running: Arc<AtomicBool>,
}

impl Dashboard {
    pub fn new(initial_session: SessionDesc) -> Self {
        Self {
            session: initial_session,
            unprocessed_events: Arc::new(Mutex::new(vec![])),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn run(&self, mut event_handler: impl FnMut(DashboardEvent)) {
        let stdout = io::stdout().into_raw_mode().unwrap();
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let key_events = Arc::new(Mutex::new(VecDeque::new()));
        thread::spawn({
            let key_events = Arc::clone(&key_events);
            let running = Arc::clone(&self.running);
            move || {
                for event in io::stdin().keys() {
                    if let Ok(event) = event {
                        key_events.lock().unwrap().push_back(event);
                    }

                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        });

        let mut selected_tab = if self.session.setup_wizard {
            4 // help tab
        } else {
            0
        };
        let mut command_mode = false;

        let mut connection_panel = ConnectionPanel::new();
        let mut installation_panel = InstallationPanel::new();
        let mut command_bar = CommandBar::new();
        let mut notification_bar = NotificationBar::new();

        while self.running.load(Ordering::Relaxed) {
            terminal
                .draw(|frame| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ])
                        .split(frame.size());
                    let tabs = Tabs::new(vec![
                        Spans::from("Connection"),
                        Spans::from("Statistics"),
                        Spans::from("Logs"),
                        Spans::from("Installation"),
                        Spans::from("Help"),
                        Spans::from("About"),
                    ])
                    .block(
                        Block::default()
                            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                            .title("ALVR Dashboard")
                            .title_alignment(Alignment::Center),
                    )
                    .select(selected_tab)
                    .highlight_style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Blue),
                    );
                    frame.render_widget(tabs, chunks[0]);

                    let panel_block = Block::default().borders(Borders::ALL);
                    let panel_area = panel_block.inner(chunks[1]);
                    frame.render_widget(panel_block, chunks[1]);

                    match selected_tab {
                        0 => connection_panel.draw(frame, panel_area, &self.session),
                        3 => installation_panel.draw(frame, panel_area),
                        4 => help::draw_help_panel(frame, panel_area),
                        5 => about::draw_about_panel(frame, panel_area),
                        _ => (),
                    }

                    if command_mode {
                        command_bar.draw(frame, chunks[2], command_mode);
                    } else {
                        notification_bar.draw(frame, chunks[2]);
                    }
                })
                .unwrap();

            while let Some(key) = key_events.lock().unwrap().pop_front() {
                match key {
                    Key::Ctrl('c') => {
                        self.running.store(false, Ordering::Relaxed);
                        event_handler(DashboardEvent::Quit);
                    }

                    key if command_mode => {
                        command_bar.react_to_key(key, &mut event_handler, &mut command_mode)
                    }

                    Key::Left => {
                        if selected_tab > 0 {
                            selected_tab -= 1;
                        }
                    }
                    Key::Right => selected_tab = cmp::min(selected_tab + 1, 5),
                    Key::Char('c') => command_mode = true,
                    key => match selected_tab {
                        0 => connection_panel.react_to_key(key, &self.session, &mut event_handler),
                        3 => installation_panel.react_to_key(key, &mut event_handler),
                        _ => (),
                    },
                }
            }
        }
    }

    pub fn update_session(&self, session_desc: SessionDesc) {}

    pub fn report_event(&self, event: ServerEvent) {}

    pub fn request_exit(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
