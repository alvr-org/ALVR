use std::{collections::VecDeque, sync::Arc};

use crate::{
    dashboard::DashboardResponse,
    translation::{SharedTranslation, TranslationBundle},
};
use alvr_events::{GraphStatistics, Statistics};
use egui::{
    emath,
    plot::{Line, Plot, PlotPoints},
    popup, pos2, vec2, Align, Align2, Color32, FontId, Frame, Id, Label, Layout, Pos2, Rect,
    RichText, Rounding, Shape, Stroke, Ui,
};

pub struct StatisticsTab {
    history: VecDeque<GraphStatistics>,
    last_statistics: Option<Statistics>,
    max_history_length: usize,
    trans: Arc<TranslationBundle>,
}

mod graph_colors {
    pub const RENDER: egui::Color32 = egui::Color32::RED;
    pub const NETWORK: egui::Color32 = egui::Color32::DARK_GRAY;
    pub const TRANSCODE: egui::Color32 = egui::Color32::BLUE;
    pub const IDLE: egui::Color32 = egui::Color32::GOLD;
    pub const SERVER_FPS: egui::Color32 = egui::Color32::GOLD;
    pub const CLIENT_FPS: egui::Color32 = egui::Color32::BLUE;
}

impl StatisticsTab {
    pub fn new(trans: Arc<TranslationBundle>) -> Self {
        Self {
            history: VecDeque::new(),
            max_history_length: 1000,
            last_statistics: None,
            trans,
        }
    }

    pub fn update_statistics(&mut self, statistics: Statistics) {
        self.last_statistics = Some(statistics);
    }

    pub fn update_graph_statistics(&mut self, statistics: GraphStatistics) {
        if self.history.len() == self.max_history_length {
            self.history.pop_back();
        }

        self.history.push_front(statistics);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<DashboardResponse> {
        ui.vertical(|ui| {
            self.draw_latency_graph(ui);
            self.draw_fps_graph(ui);
            self.draw_statistics_overview(ui);
        });

        None
    }

    fn draw_latency_graph(&self, ui: &mut Ui) {
        let mut from_screen = None;
        ui.add_space(10.0);
        ui.label(RichText::new(self.trans.get("latency")).size(20.0));
        match Frame::canvas(ui.style())
            .show(ui, |ui| {
                ui.ctx().request_repaint();
                let size = ui.available_width() * vec2(1.0, 0.3);

                let (_id, rect) = ui.allocate_space(size);

                let max = self
                    .history
                    .iter()
                    .map(|graph| graph.total_pipeline_latency_s as i32 + 20)
                    .max()
                    .unwrap_or(0);

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=self.max_history_length as f32, max as f32..=0.0),
                    rect,
                );

                from_screen = Some(emath::RectTransform::from_to(
                    rect,
                    Rect::from_x_y_ranges(0.0..=self.max_history_length as f32, max as f32..=0.0),
                ));

                for i in 0..self.max_history_length {
                    match self.history.get(i) {
                        Some(graph) => {
                            let mut offset = 0.0;
                            for (value, color) in &[
                                (graph.game_time_s, graph_colors::RENDER),
                                (graph.server_compositor_s, graph_colors::IDLE),
                                (graph.encoder_s, graph_colors::TRANSCODE),
                                (graph.network_s, graph_colors::NETWORK),
                                (graph.decoder_s, graph_colors::TRANSCODE),
                                (graph.client_compositor_s, graph_colors::IDLE),
                            ] {
                                ui.painter().rect_filled(
                                    Rect {
                                        min: to_screen
                                            * pos2(
                                                (self.max_history_length - i) as f32,
                                                offset + value,
                                            ),
                                        max: to_screen
                                            * pos2(
                                                (self.max_history_length - i) as f32 + 2.0,
                                                offset,
                                            ),
                                    },
                                    Rounding::none(),
                                    *color,
                                );
                                offset += value;
                            }
                        }
                        None => (),
                    }
                }

                ui.painter().text(
                    to_screen * pos2(0.0, 0.0),
                    Align2::LEFT_BOTTOM,
                    format!("0"),
                    FontId::monospace(20.0),
                    Color32::GRAY,
                );
                ui.painter().text(
                    to_screen * pos2(0.0, max as f32),
                    Align2::LEFT_TOP,
                    format!("{}", max),
                    FontId::monospace(20.0),
                    Color32::GRAY,
                );
            })
            .response
            .hover_pos()
        {
            Some(pos) => {
                popup::show_tooltip(ui.ctx(), Id::new("latency_graph_popup"), |ui| {
                    let graph_pos = from_screen.unwrap() * pos;

                    match self
                        .history
                        .get(self.max_history_length - graph_pos.x as usize)
                    {
                        Some(graph) => {
                            ui.label(&format!(
                                "Total latency: {:.2}ms",
                                graph.total_pipeline_latency_s
                            ));
                            ui.colored_label(
                                graph_colors::IDLE,
                                &format!("Client compositor: {:.2}ms", graph.client_compositor_s),
                            );
                            ui.colored_label(
                                graph_colors::TRANSCODE,
                                &format!("Decode: {:.2}ms", graph.decoder_s),
                            );
                            ui.colored_label(
                                graph_colors::NETWORK,
                                &format!("Network: {:.2}ms", graph.network_s),
                            );
                            ui.colored_label(
                                graph_colors::TRANSCODE,
                                &format!("Encode: {:.2}ms", graph.encoder_s),
                            );
                            ui.colored_label(
                                graph_colors::IDLE,
                                &format!("Server compositor: {:.2}ms", graph.server_compositor_s),
                            );
                            ui.colored_label(
                                graph_colors::RENDER,
                                &format!("Render: {:.2}ms", graph.game_time_s),
                            );
                        }
                        None => {}
                    }
                });
            }
            None => (),
        }
        ui.horizontal(|ui| {
            ui.colored_label(graph_colors::IDLE, "Client compositor");
            ui.colored_label(graph_colors::TRANSCODE, "Decode");
            ui.colored_label(graph_colors::NETWORK, "Network");
            ui.colored_label(graph_colors::TRANSCODE, "Encode");
            ui.colored_label(graph_colors::IDLE, "Server compositor");
            ui.colored_label(graph_colors::RENDER, "Render");
        });
    }

    fn draw_fps_graph(&self, ui: &mut Ui) {
        let mut from_screen = None;

        ui.add_space(10.0);
        ui.label(RichText::new("FPS").size(20.0));
        match Frame::canvas(ui.style())
            .show(ui, |ui| {
                ui.ctx().request_repaint();
                let size = ui.available_width() * vec2(1.0, 0.3);

                let (_id, rect) = ui.allocate_space(size);

                let max = self
                    .history
                    .iter()
                    .map(|graph| graph.client_fps.max(graph.server_fps) as i32 + 10)
                    .max()
                    .unwrap_or(100);
                let min = self
                    .history
                    .iter()
                    .map(|graph| graph.client_fps.min(graph.server_fps) as i32 - 10)
                    .min()
                    .unwrap_or(0);

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(
                        0.0..=self.max_history_length as f32,
                        max as f32..=min as f32,
                    ),
                    rect,
                );

                from_screen = Some(emath::RectTransform::from_to(
                    rect,
                    Rect::from_x_y_ranges(
                        0.0..=self.max_history_length as f32,
                        max as f32..=min as f32,
                    ),
                ));

                let (client_fps_points, server_fps_points): (Vec<Pos2>, Vec<Pos2>) = (0..self
                    .max_history_length)
                    .map(|i| match self.history.get(i) {
                        Some(graph) => (
                            to_screen
                                * pos2((self.max_history_length - i) as f32, graph.client_fps),
                            to_screen
                                * pos2((self.max_history_length - i) as f32, graph.server_fps),
                        ),
                        None => (
                            to_screen * pos2((self.max_history_length - i) as f32, 0.0),
                            to_screen * pos2((self.max_history_length - i) as f32, 0.0),
                        ),
                    })
                    .unzip();

                ui.painter().add(Shape::line(
                    client_fps_points,
                    Stroke::new(1.0, graph_colors::CLIENT_FPS),
                ));
                ui.painter().add(Shape::line(
                    server_fps_points,
                    Stroke::new(1.0, graph_colors::SERVER_FPS),
                ));
                ui.painter().text(
                    to_screen * pos2(0.0, min as f32),
                    Align2::LEFT_BOTTOM,
                    format!("{}", min),
                    FontId::monospace(20.0),
                    Color32::GRAY,
                );
                ui.painter().text(
                    to_screen * pos2(0.0, max as f32),
                    Align2::LEFT_TOP,
                    format!("{}", max),
                    FontId::monospace(20.0),
                    Color32::GRAY,
                );
            })
            .response
            .hover_pos()
        {
            Some(pos) => {
                popup::show_tooltip(ui.ctx(), Id::new("client_server_fps_popup"), |ui| {
                    let graph_pos = from_screen.unwrap() * pos;

                    match self
                        .history
                        .get(self.max_history_length - graph_pos.x as usize)
                    {
                        Some(graph) => {
                            ui.colored_label(
                                graph_colors::CLIENT_FPS,
                                format!("Client FPS: {:.2}", graph.client_fps),
                            );
                            ui.colored_label(
                                graph_colors::SERVER_FPS,
                                format!("Server FPS: {:.2}", graph.server_fps),
                            );
                        }
                        None => (),
                    }
                });
            }
            None => (),
        }
        ui.horizontal(|ui| {
            ui.colored_label(graph_colors::CLIENT_FPS, "Client FPS");
            ui.colored_label(graph_colors::SERVER_FPS, "Server FPS");
        });
    }

    fn draw_statistics_overview(&self, ui: &mut Ui) {
        let statistics = self.last_statistics.clone().unwrap_or_default();
        ui.columns(2, |ui| {
            ui[0].label("Total packets:");
            ui[1].label(&format!(
                "{} packets ({} packets/s)",
                statistics.video_packets_total, statistics.video_packets_per_sec
            ));
            //ui[0].label("Total packets lost:");
            ui[0].label("Total sent:");
            ui[1].label(&format!("{} packets", statistics.video_packets_total));

            ui[0].label("Total sent:");
            ui[1].label(&format!("{} MB", statistics.video_mbytes_total));

            ui[0].label("Bitrate:");
            ui[1].label(&format!("{} Mbps", statistics.video_mbits_per_sec));

            //ui[0].label("Ping:");
            ui[0].label("Total latency:");
            ui[1].label(&format!("{} ms", statistics.total_latency_ms));

            ui[0].label("Encoder latency:");
            ui[1].label(&format!("{} ms", statistics.encode_latency_ms));

            ui[0].label("Transport latency:");
            ui[1].label(&format!("{} ms", statistics.network_latency_ms));

            ui[0].label("Decoder latency:");
            ui[1].label(&format!("{} ms", statistics.decode_latency_ms));

            ui[0].label("Fec percentage:");
            ui[1].label(&format!("{} %", statistics.fec_percentage));

            ui[0].label("Fec failure total:");
            ui[1].label(&format!(
                "{} packets ({} packets/s)",
                statistics.fec_errors_total, statistics.fec_errors_per_sec
            ));

            ui[0].label("Client FPS:");
            ui[1].label(&format!("{} FPS", statistics.client_fps));

            ui[0].label("Server FPS:");
            ui[1].label(&format!("{} FPS", statistics.server_fps));
        });
    }
}
