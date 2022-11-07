use std::collections::VecDeque;

use crate::dashboard::DashboardResponse;
use alvr_events::LogEvent;
use egui::{ScrollArea, Ui};

pub struct LogsTab {
    logs: VecDeque<LogEvent>,
    log_limit: usize,
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            logs: VecDeque::new(),
            log_limit: 1000,
        }
    }

    pub fn update_logs(&mut self, log: LogEvent) {
        if self.logs.len() >= self.log_limit {
            self.logs.pop_front();
        }
        self.logs.push_back(log);
    }

    pub fn ui(&self, ui: &mut Ui) -> Option<DashboardResponse> {
        ScrollArea::vertical().show(ui, |ui| {
            for log in &self.logs {
                ui.horizontal(|ui| {
                    ui.monospace(&log.timestamp);
                    ui.monospace(&log.content);
                });
            }
        });

        None
    }
}
