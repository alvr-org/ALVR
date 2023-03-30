use std::collections::VecDeque;

use crate::{dashboard::theme::graph_colors, dashboard::DashboardRequest};
use alvr_events::{GraphStatistics, Statistics};
use eframe::egui::{
    emath, popup, pos2, vec2, Align2, Color32, FontId, Frame, Id, Pos2, Rect, RichText, Rounding,
    Shape, Stroke, Ui,
};

fn legend_label(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(10.0).color(color));
}

pub struct StatisticsTab {
    history: VecDeque<GraphStatistics>,
    last_statistics: Option<Statistics>,
    max_history_length: usize,
}

impl StatisticsTab {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            max_history_length: 1000,
            last_statistics: None,
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

    pub fn ui(&mut self, ui: &mut Ui) -> Option<DashboardRequest> {
        ui.vertical(|ui| {
            let avaibale_width = ui.available_width();
            self.draw_latency_graph(ui, avaibale_width);
            self.draw_fps_graph(ui, avaibale_width);
            self.draw_statistics_overview(ui);
        });

        None
    }

    fn draw_latency_graph(&self, ui: &mut Ui, available_width: f32) {
        ui.add_space(10.0);
        ui.label(RichText::new("Latency").size(20.0));

        let mut from_screen = None;
        let canvas_response = Frame::canvas(ui.style())
            .show(ui, |ui| {
                ui.ctx().request_repaint();
                let size = available_width * vec2(1.0, 0.2);

                let (_id, rect) = ui.allocate_space(size);

                let max = self
                    .history
                    .iter()
                    .map(|graph| (graph.total_pipeline_latency_s * 1000.0) as i32 + 20)
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
                    if let Some(stats) = self.history.get(i) {
                        let mut offset = 0.0;
                        for (value, color) in &[
                            (stats.game_time_s, graph_colors::RENDER_VARIANT),
                            (stats.server_compositor_s, graph_colors::RENDER),
                            (stats.encoder_s, graph_colors::TRANSCODE),
                            (stats.network_s, graph_colors::NETWORK),
                            (stats.decoder_s, graph_colors::TRANSCODE),
                            (stats.decoder_queue_s, graph_colors::IDLE),
                            (stats.client_compositor_s, graph_colors::RENDER),
                            (stats.vsync_queue_s, graph_colors::IDLE),
                        ] {
                            ui.painter().rect_filled(
                                Rect {
                                    min: to_screen
                                        * pos2(
                                            (self.max_history_length - i) as f32,
                                            offset + value * 1000.0,
                                        ),
                                    max: to_screen
                                        * pos2((self.max_history_length - i) as f32 + 2.0, offset),
                                },
                                Rounding::none(),
                                *color,
                            );
                            offset += value * 1000.0;
                        }
                    }
                }

                ui.painter().text(
                    to_screen * pos2(0.0, 0.0),
                    Align2::LEFT_BOTTOM,
                    "0".to_string(),
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
            .response;

        if let Some(pos) = canvas_response.hover_pos() {
            let mut graph_pos = from_screen.unwrap() * pos;
            graph_pos.x = graph_pos.x.min(self.max_history_length as f32);

            if let Some(stats) = self
                .history
                .get(self.max_history_length - graph_pos.x as usize)
            {
                popup::show_tooltip(ui.ctx(), Id::new("latency_graph_popup"), |ui| {
                    ui.label(&format!(
                        "Total latency: {:.2}ms",
                        stats.total_pipeline_latency_s * 1000.0
                    ));
                    ui.colored_label(
                        graph_colors::IDLE,
                        &format!("Client VSync: {:.2}ms", stats.vsync_queue_s * 1000.0),
                    );
                    ui.colored_label(
                        graph_colors::RENDER,
                        &format!(
                            "Client compositor: {:.2}ms",
                            stats.client_compositor_s * 1000.0
                        ),
                    );
                    ui.colored_label(
                        graph_colors::IDLE,
                        &format!("Decoder queue: {:.2}ms", stats.decoder_queue_s * 1000.0),
                    );
                    ui.colored_label(
                        graph_colors::TRANSCODE,
                        &format!("Decode: {:.2}ms", stats.decoder_s * 1000.0),
                    );
                    ui.colored_label(
                        graph_colors::NETWORK,
                        &format!("Network: {:.2}ms", stats.network_s * 1000.0),
                    );
                    ui.colored_label(
                        graph_colors::TRANSCODE,
                        &format!("Encode: {:.2}ms", stats.encoder_s * 1000.0),
                    );
                    ui.colored_label(
                        graph_colors::RENDER,
                        &format!(
                            "Streamer compositor: {:.2}ms",
                            stats.server_compositor_s * 1000.0
                        ),
                    );
                    ui.colored_label(
                        graph_colors::RENDER_VARIANT,
                        &format!("Game render: {:.2}ms", stats.game_time_s * 1000.0),
                    );
                });
            }
        }

        ui.horizontal(|ui| {
            legend_label(ui, "Game render", graph_colors::RENDER_VARIANT);
            legend_label(ui, "Streamer compositor", graph_colors::RENDER);
            legend_label(ui, "Encode", graph_colors::TRANSCODE);
            legend_label(ui, "Network", graph_colors::NETWORK);
            legend_label(ui, "Decode", graph_colors::TRANSCODE);
            legend_label(ui, "Decoder queue", graph_colors::IDLE);
            legend_label(ui, "Client compositor", graph_colors::RENDER);
            legend_label(ui, "Client VSync", graph_colors::IDLE);
        });
    }

    fn draw_fps_graph(&self, ui: &mut Ui, available_width: f32) {
        ui.add_space(10.0);
        ui.label(RichText::new("FPS").size(20.0));

        let mut from_screen = None;
        let canvas_response = Frame::canvas(ui.style())
            .show(ui, |ui| {
                ui.ctx().request_repaint();
                let size = available_width * vec2(1.0, 0.2);

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

                let (server_fps_points, client_fps_points): (Vec<Pos2>, Vec<Pos2>) = (0..self
                    .max_history_length)
                    .map(|i| match self.history.get(i) {
                        Some(graph) => (
                            to_screen
                                * pos2((self.max_history_length - i) as f32, graph.server_fps),
                            to_screen
                                * pos2((self.max_history_length - i) as f32, graph.client_fps),
                        ),
                        None => (
                            to_screen * pos2((self.max_history_length - i) as f32, 0.0),
                            to_screen * pos2((self.max_history_length - i) as f32, 0.0),
                        ),
                    })
                    .unzip();

                ui.painter().add(Shape::line(
                    server_fps_points,
                    Stroke::new(1.0, graph_colors::SERVER_FPS),
                ));
                ui.painter().add(Shape::line(
                    client_fps_points,
                    Stroke::new(1.0, graph_colors::CLIENT_FPS),
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
            .response;

        if let Some(pos) = canvas_response.hover_pos() {
            let mut graph_pos = from_screen.unwrap() * pos;
            graph_pos.x = graph_pos.x.min(self.max_history_length as f32);

            if let Some(stats) = self
                .history
                .get(self.max_history_length - graph_pos.x as usize)
            {
                popup::show_tooltip(ui.ctx(), Id::new("client_server_fps_popup"), |ui| {
                    ui.colored_label(
                        graph_colors::SERVER_FPS,
                        format!("Streamer FPS: {:.2}", stats.server_fps),
                    );
                    ui.colored_label(
                        graph_colors::CLIENT_FPS,
                        format!("Client FPS: {:.2}", stats.client_fps),
                    );
                });
            }
        }
        ui.horizontal(|ui| {
            legend_label(ui, "Streamer FPS", graph_colors::SERVER_FPS);
            legend_label(ui, "Client FPS", graph_colors::CLIENT_FPS);
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

            ui[0].label("Total sent:");
            ui[1].label(&format!("{} MB", statistics.video_mbytes_total));

            ui[0].label("Bitrate:");
            ui[1].label(&format!("{} Mbps", statistics.video_mbits_per_sec));

            ui[0].label("Total latency:");
            ui[1].label(&format!("{:.2} ms", statistics.total_latency_ms));

            ui[0].label("Encoder latency:");
            ui[1].label(&format!("{:.2} ms", statistics.encode_latency_ms));

            ui[0].label("Transport latency:");
            ui[1].label(&format!("{:.2} ms", statistics.network_latency_ms));

            ui[0].label("Decoder latency:");
            ui[1].label(&format!("{:.2} ms", statistics.decode_latency_ms));

            ui[0].label("Total packets lost:");
            ui[1].label(&format!(
                "{} packets ({} packets/s)",
                statistics.packets_lost_total, statistics.packets_lost_per_sec
            ));

            ui[0].label("Client FPS:");
            ui[1].label(&format!("{} FPS", statistics.client_fps));

            ui[0].label("Streamer FPS:");
            ui[1].label(&format!("{} FPS", statistics.server_fps));

            ui[0].label("Headset battery");
            ui[1].label(&format!("{}%", statistics.battery_hmd));
        });
    }
}
