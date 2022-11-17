use std::collections::VecDeque;

use crate::dashboard::DashboardResponse;
use alvr_events::Event;
use egui::{ScrollArea, Ui};

pub struct LogsTab {
    events: VecDeque<Event>,
    log_limit: usize,
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            log_limit: 1000,
        }
    }

    pub fn update_logs(&mut self, event: Event) {
        if self.events.len() >= self.log_limit {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn ui(&self, ui: &mut Ui) -> Option<DashboardResponse> {
        ui.centered_and_justified(|ui| {
            ScrollArea::both().show(ui, |ui| {
                for event in &self.events {
                    ui.horizontal(|ui| {
                        ui.monospace(&event.timestamp);
                        ui.monospace(&format!("{:?}", event.event_type));
                    });
                }
            });
        });

        None
    }
}
