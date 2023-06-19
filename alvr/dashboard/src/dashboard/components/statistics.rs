use std::{collections::VecDeque, ops::RangeInclusive};

use crate::{dashboard::theme::graph_colors, dashboard::ServerRequest, theme};
use alvr_events::{GraphStatistics, StatisticsSummary};
use eframe::{
    egui::{
        popup, pos2, vec2, Align2, Color32, FontId, Frame, Id, Rect, RichText, Rounding, Shape,
        Stroke, Ui,
    },
    emath::RectTransform,
    epaint::Pos2,
};

const GRAPH_HISTORY_SIZE: usize = 1000;

fn draw_lines(ui: &mut Ui, points: Vec<Pos2>, color: Color32) {
    ui.painter()
        .add(Shape::line(points, Stroke::new(1.0, color)));
}

pub struct StatisticsTab {
    history: VecDeque<GraphStatistics>,
    last_statistics_summary: Option<StatisticsSummary>,
}

impl StatisticsTab {
    pub fn new() -> Self {
        Self {
            history: vec![GraphStatistics::default(); GRAPH_HISTORY_SIZE]
                .into_iter()
                .collect(),
            last_statistics_summary: None,
        }
    }

    pub fn update_statistics(&mut self, statistics: StatisticsSummary) {
        self.last_statistics_summary = Some(statistics);
    }

    pub fn update_graph_statistics(&mut self, statistics: GraphStatistics) {
        self.history.pop_front();
        self.history.push_back(statistics);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<ServerRequest> {
        if let Some(stats) = &self.last_statistics_summary {
            ui.vertical(|ui| {
                let available_width = ui.available_width();
                self.draw_latency_graph(ui, available_width);
                self.draw_fps_graph(ui, available_width);
                self.draw_bitrate_graph(ui, available_width);
                self.draw_statistics_overview(ui, stats);
            });
        } else {
            ui.heading("No statistics available");
        }

        None
    }

    fn draw_graph(
        &self,
        ui: &mut Ui,
        available_width: f32,
        title: &str,
        data_range: RangeInclusive<f32>,
        graph_content: impl FnOnce(&mut Ui, RectTransform),
        tooltip_content: impl FnOnce(&mut Ui, &GraphStatistics),
    ) {
        ui.add_space(10.0);
        ui.label(RichText::new(title).size(20.0));

        let canvas_response = Frame::canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let size = available_width * vec2(1.0, 0.2);

            let (_id, rect) = ui.allocate_space(size);

            let max = data_range.end() * 1.1;
            let min = data_range.start() * 0.9;
            let data_rect = Rect::from_x_y_ranges(0.0..=GRAPH_HISTORY_SIZE as f32, max..=min);
            let to_screen = RectTransform::from_to(data_rect, rect);

            graph_content(ui, to_screen);

            ui.painter().text(
                to_screen * pos2(0.0, min),
                Align2::LEFT_BOTTOM,
                format!("{:.0}", min),
                FontId::monospace(20.0),
                Color32::GRAY,
            );
            ui.painter().text(
                to_screen * pos2(0.0, max),
                Align2::LEFT_TOP,
                format!("{:.0}", max),
                FontId::monospace(20.0),
                Color32::GRAY,
            );

            data_rect
        });

        if let Some(pos) = canvas_response.response.hover_pos() {
            let graph_pos =
                RectTransform::from_to(canvas_response.response.rect, canvas_response.inner) * pos;

            popup::show_tooltip(ui.ctx(), Id::new("popup"), |ui| {
                tooltip_content(ui, self.history.get(graph_pos.x as usize).unwrap())
            });
        }
    }

    fn draw_latency_graph(&self, ui: &mut Ui, available_width: f32) {
        let max = self
            .history
            .iter()
            .map(|stats| stats.total_pipeline_latency_s)
            .reduce(f32::max)
            .unwrap_or(0.0)
            * 1000.0;

        self.draw_graph(
            ui,
            available_width,
            "Latency",
            0.0..=max,
            |ui, to_screen_trans| {
                for i in 0..GRAPH_HISTORY_SIZE {
                    let stats = self.history.get(i).unwrap();
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
                                min: to_screen_trans * pos2(i as f32, offset + value * 1000.0),
                                max: to_screen_trans * pos2(i as f32 + 2.0, offset),
                            },
                            Rounding::none(),
                            *color,
                        );
                        offset += value * 1000.0;
                    }
                }
            },
            |ui, stats| {
                use graph_colors::*;

                fn label(ui: &mut Ui, text: &str, value_s: f32, color: Color32) {
                    ui.colored_label(color, &format!("{text}: {:.2}ms", value_s * 1000.0));
                }

                label(
                    ui,
                    "Total latency",
                    stats.total_pipeline_latency_s,
                    theme::FG,
                );
                label(ui, "Client VSync", stats.vsync_queue_s, IDLE);
                label(ui, "Client compositor", stats.client_compositor_s, RENDER);
                label(ui, "Decoder queue", stats.decoder_queue_s, IDLE);
                label(ui, "Decode", stats.decoder_s, TRANSCODE);
                label(ui, "Network", stats.network_s, NETWORK);
                label(ui, "Encode", stats.encoder_s, TRANSCODE);
                label(ui, "Streamer compositor", stats.server_compositor_s, RENDER);
                label(ui, "Game render", stats.game_time_s, RENDER_VARIANT);
            },
        );
    }

    fn draw_fps_graph(&self, ui: &mut Ui, available_width: f32) {
        let min = self
            .history
            .iter()
            .map(|stats| f32::min(stats.client_fps, stats.server_fps))
            .reduce(f32::min)
            .unwrap();
        let max = self
            .history
            .iter()
            .map(|stats| f32::max(stats.client_fps, stats.server_fps))
            .reduce(f32::max)
            .unwrap();

        self.draw_graph(
            ui,
            available_width,
            "Framerate",
            min..=max,
            |ui, to_screen_trans| {
                let (server_fps_points, client_fps_points) = (0..GRAPH_HISTORY_SIZE)
                    .map(|i| {
                        (
                            to_screen_trans * pos2(i as f32, self.history[i].server_fps),
                            to_screen_trans * pos2(i as f32, self.history[i].client_fps),
                        )
                    })
                    .unzip();

                draw_lines(ui, server_fps_points, graph_colors::SERVER_FPS);
                draw_lines(ui, client_fps_points, graph_colors::CLIENT_FPS);
            },
            |ui, stats| {
                ui.colored_label(
                    graph_colors::SERVER_FPS,
                    format!("Streamer FPS: {:.2}", stats.server_fps),
                );
                ui.colored_label(
                    graph_colors::CLIENT_FPS,
                    format!("Client FPS: {:.2}", stats.client_fps),
                );
            },
        );
    }

    fn draw_bitrate_graph(&self, ui: &mut Ui, available_width: f32) {
        let max = self
            .history
            .iter()
            .map(|stats| {
                // Note: skip max_from_decoder_latency_limiter because might be inf
                let nom_br = &stats.nominal_bitrate;
                let nominal_max = [
                    &nom_br.scaled_calculated_bps,
                    &nom_br.network_latency_limiter_bps,
                    &nom_br.encoder_latency_limiter_bps,
                    &nom_br.manual_max_bps,
                    &nom_br.manual_min_bps,
                ]
                .iter()
                .fold(nom_br.requested_bps, |acc, val| {
                    val.map(|val| f32::max(acc, val)).unwrap_or(acc)
                });

                f32::max(nominal_max, stats.actual_bitrate_bps)
            })
            .reduce(f32::max)
            .unwrap()
            / 1e6;
        let min = self
            .history
            .iter()
            .map(|stats| {
                let nom_br = &stats.nominal_bitrate;
                let nominal_min = [
                    &nom_br.scaled_calculated_bps,
                    &nom_br.network_latency_limiter_bps,
                    &nom_br.encoder_latency_limiter_bps,
                    &nom_br.manual_max_bps,
                    &nom_br.manual_min_bps,
                ]
                .iter()
                .fold(nom_br.requested_bps, |acc, val| {
                    val.map(|val| f32::min(acc, val)).unwrap_or(acc)
                });

                f32::min(nominal_min, stats.actual_bitrate_bps)
            })
            .reduce(f32::min)
            .unwrap()
            / 1e6;

        self.draw_graph(
            ui,
            available_width,
            "Bitrate",
            min..=max,
            |ui, to_screen_trans| {
                let mut scaled_calculated = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut decoder_latency_limiter = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut network_latency_limiter = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut encoder_latency_limiter = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut manual_max = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut manual_min = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut requested = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                let mut actual = Vec::with_capacity(GRAPH_HISTORY_SIZE);
                for i in 0..GRAPH_HISTORY_SIZE {
                    let nom_br = &self.history[i].nominal_bitrate;

                    if let Some(value) = nom_br.scaled_calculated_bps {
                        scaled_calculated.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    if let Some(value) = nom_br.decoder_latency_limiter_bps {
                        decoder_latency_limiter.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    if let Some(value) = nom_br.network_latency_limiter_bps {
                        network_latency_limiter.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    if let Some(value) = nom_br.encoder_latency_limiter_bps {
                        encoder_latency_limiter.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    if let Some(value) = nom_br.manual_max_bps {
                        manual_max.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    if let Some(value) = nom_br.manual_min_bps {
                        manual_min.push(to_screen_trans * pos2(i as f32, value / 1e6))
                    }
                    requested.push(to_screen_trans * pos2(i as f32, nom_br.requested_bps / 1e6));
                    actual.push(
                        to_screen_trans * pos2(i as f32, self.history[i].actual_bitrate_bps / 1e6),
                    );
                }

                draw_lines(ui, scaled_calculated, Color32::GRAY);
                draw_lines(ui, encoder_latency_limiter, graph_colors::TRANSCODE);
                draw_lines(ui, network_latency_limiter, graph_colors::NETWORK);
                draw_lines(ui, decoder_latency_limiter, graph_colors::TRANSCODE);
                draw_lines(ui, manual_max, graph_colors::RENDER);
                draw_lines(ui, manual_min, graph_colors::RENDER);
                draw_lines(ui, requested, theme::OK_GREEN);
                draw_lines(ui, actual, theme::FG);
            },
            |ui, stats| {
                fn label(ui: &mut Ui, text: &str, maybe_value_bps: Option<f32>, color: Color32) {
                    if let Some(value) = maybe_value_bps {
                        ui.colored_label(color, &format!("{text}: {:.2} Mbps", value / 1e6));
                    }
                }

                let n = &stats.nominal_bitrate;

                label(
                    ui,
                    "Initial calculated",
                    n.scaled_calculated_bps,
                    Color32::GRAY,
                );
                label(
                    ui,
                    "Encoder latency limiter",
                    n.encoder_latency_limiter_bps,
                    graph_colors::TRANSCODE,
                );
                label(
                    ui,
                    "Network latency limiter",
                    n.network_latency_limiter_bps,
                    graph_colors::NETWORK,
                );
                label(
                    ui,
                    "Decoder latency limiter",
                    n.decoder_latency_limiter_bps,
                    graph_colors::TRANSCODE,
                );
                label(ui, "Manual max", n.manual_max_bps, graph_colors::RENDER);
                label(ui, "Manual min", n.manual_min_bps, graph_colors::RENDER);
                label(ui, "Requested", Some(n.requested_bps), theme::OK_GREEN);
                label(
                    ui,
                    "Actual recorded",
                    Some(stats.actual_bitrate_bps),
                    theme::FG,
                );
            },
        )
    }

    fn draw_statistics_overview(&self, ui: &mut Ui, statistics: &StatisticsSummary) {
        ui.add_space(10.0);

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
            ui[1].label(&format!(
                "{}% ({})",
                statistics.battery_hmd,
                if statistics.hmd_plugged {
                    "plugged"
                } else {
                    "unplugged"
                }
            ));
        });
    }
}
