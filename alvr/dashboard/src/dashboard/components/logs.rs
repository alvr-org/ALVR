use alvr_common::LogSeverity;
use alvr_events::{Event, EventType};
use alvr_gui_common::theme::log_colors;
use alvr_session::{RawEventsConfig, Settings};
use eframe::{
    egui::{Grid, OpenUrl, RichText, ScrollArea, Ui},
    epaint::Color32,
};
use settings_schema::Switch;
use std::{collections::VecDeque, env};

struct Entry {
    color: Color32,
    timestamp: String,
    ty: String,
    message: String,
}

pub struct LogsTab {
    raw_events_config: Switch<RawEventsConfig>,
    entries: VecDeque<Entry>,
    log_limit: usize,
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            raw_events_config: Switch::Enabled(RawEventsConfig {
                hide_spammy_events: false,
            }),
            entries: VecDeque::new(),
            log_limit: 1000,
        }
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.raw_events_config = settings.logging.show_raw_events.clone();
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
                if let Switch::Enabled(config) = &self.raw_events_config {
                    if !config.hide_spammy_events
                        || !matches!(
                            event_type,
                            EventType::StatisticsSummary(_)
                                | EventType::GraphStatistics(_)
                                | EventType::Tracking(_)
                        )
                    {
                        self.entries.push_back(Entry {
                            color: log_colors::EVENT_LIGHT,
                            timestamp: event.timestamp,
                            ty: "EVENT".into(),
                            message: format!("{event_type:?}"),
                        });
                    }
                }
            }
        }

        if self.entries.len() > self.log_limit {
            self.entries.pop_front();
        }
    }

    pub fn ui(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Copy all").clicked() {
                ui.output_mut(|out| {
                    out.copied_text = self.entries.iter().fold(String::new(), |acc, entry| {
                        format!(
                            "{}{} [{}] {}\n",
                            acc, entry.timestamp, entry.ty, entry.message
                        )
                    })
                })
            }
            if ui.button("Open logs directory").clicked() {
                let log_dir = alvr_filesystem::filesystem_layout_from_dashboard_exe(
                    &env::current_exe().unwrap(),
                )
                .log_dir;
                ui.output_mut(|f| {
                    f.open_url = Some(OpenUrl::same_tab(format!(
                        "file://{}",
                        log_dir.to_string_lossy()
                    )))
                });
            }
        });

        ScrollArea::both()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Grid::new(0)
                    .spacing((10.0, 2.0))
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in &self.entries {
                            ui.colored_label(
                                entry.color,
                                RichText::new(&entry.timestamp).size(12.0),
                            );
                            ui.colored_label(entry.color, RichText::new(&entry.ty).size(12.0));
                            ui.colored_label(entry.color, RichText::new(&entry.message).size(12.0));

                            ui.end_row();
                        }
                    });
            });
    }
}
