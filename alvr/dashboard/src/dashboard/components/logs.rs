use std::collections::VecDeque;

use crate::theme::log_colors;
use alvr_common::LogSeverity;
use alvr_events::{Event, EventType};
use alvr_session::Settings;
use eframe::{
    egui::{Grid, ScrollArea, Ui},
    epaint::Color32,
};

struct Entry {
    color: Color32,
    timestamp: String,
    ty: String,
    message: String,
}

pub struct LogsTab {
    show_raw_events: bool,
    entries: VecDeque<Entry>,
    log_limit: usize,
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            show_raw_events: true,
            entries: VecDeque::new(),
            log_limit: 1000,
        }
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.show_raw_events = settings.extra.show_raw_events;
    }

    pub fn push_event(&mut self, event: Event) {
        match event.event_type {
            EventType::Log(log_event) => {
                let color;
                let ty;
                match log_event.severity {
                    LogSeverity::Error => {
                        color = log_colors::ERROR_LIGHT;
                        ty = "ERROR";
                    }
                    LogSeverity::Warning => {
                        color = log_colors::WARNING_LIGHT;
                        ty = "WARN";
                    }
                    LogSeverity::Info => {
                        color = log_colors::INFO_LIGHT;
                        ty = "INFO";
                    }
                    LogSeverity::Debug => {
                        color = log_colors::DEBUG_LIGHT;
                        ty = "DEBUG";
                    }
                };

                self.entries.push_back(Entry {
                    color,
                    timestamp: event.timestamp,
                    ty: ty.into(),
                    message: log_event.content,
                });
            }
            event_type => {
                if self.show_raw_events {
                    self.entries.push_back(Entry {
                        color: log_colors::EVENT_LIGHT,
                        timestamp: event.timestamp,
                        ty: "EVENT".into(),
                        message: format!("{event_type:?}"),
                    });
                }
            }
        }

        if self.entries.len() > self.log_limit {
            self.entries.pop_front();
        }
    }

    pub fn ui(&self, ui: &mut Ui) {
        ScrollArea::both().show(ui, |ui| {
            Grid::new(0).num_columns(3).striped(true).show(ui, |ui| {
                for entry in &self.entries {
                    ui.colored_label(entry.color, &entry.timestamp);
                    ui.colored_label(entry.color, &entry.ty);
                    ui.colored_label(entry.color, &entry.message);

                    ui.end_row();
                }
            });
        });
    }
}
