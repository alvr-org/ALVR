use crate::dashboard::{DashboardEvent, DriverRegistrationEvent, FirewallRulesEvent};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState},
    Frame,
};

pub struct InstallationPanel {
    list_state: ListState,
}

impl InstallationPanel {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let list = List::new(vec![
            ListItem::new("Add firewall rules"),
            ListItem::new("Remove firewall rules"),
            ListItem::new("Register driver"),
            ListItem::new("Unregister driver"),
        ])
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    pub fn react_to_key(&mut self, key: Key, event_handler: &mut impl FnMut(DashboardEvent)) {
        match key {
            Key::Up => {
                let selection = self.list_state.selected().unwrap();
                if selection > 0 {
                    self.list_state.select(Some(selection - 1));
                }
            }
            Key::Down => {
                let selection = self.list_state.selected().unwrap();
                if selection < 3 {
                    self.list_state.select(Some(selection + 1));
                }
            }
            Key::Char('\n') => match self.list_state.selected().unwrap() {
                0 => event_handler(DashboardEvent::FirewallRules(FirewallRulesEvent::Add)),
                1 => event_handler(DashboardEvent::FirewallRules(FirewallRulesEvent::Remove)),
                2 => event_handler(DashboardEvent::Driver(
                    DriverRegistrationEvent::RegisterAlvr,
                )),
                3 => event_handler(DashboardEvent::Driver(DriverRegistrationEvent::Unregister(
                    "".into(),
                ))), // todo: get path
                _ => unreachable!(),
            },
            _ => (),
        }
    }
}
