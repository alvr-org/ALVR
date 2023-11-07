use crate::{dashboard::theme::graph_colors, dashboard::ServerRequest};
use alvr_events::{GraphStatistics, StatisticsSummary};
use alvr_gui_common::theme;
use eframe::{
    egui::{
        popup, pos2, vec2, Align2, Color32, FontId, Frame, Id, Painter, Rect, RichText, Rounding,
        ScrollArea, Shape, Stroke, Ui,
    },
    emath::RectTransform,
    epaint::Pos2,
};
use statrs::statistics::{self, OrderStatistics};
use std::{collections::VecDeque, ops::RangeInclusive};

const GRAPH_HISTORY_SIZE: usize = 1000;
const UPPER_QUANTILE: f64 = 0.90;

fn draw_lines(painter: &Painter, points: Vec<Pos2>, color: Color32) {
    painter.add(Shape::line(points, Stroke::new(1.0, color)));
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
            ScrollArea::new([false, true]).show(ui, |ui| {
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
        graph_content: impl FnOnce(&Painter, RectTransform),
        tooltip_content: impl FnOnce(&mut Ui, &GraphStatistics),
    ) {
        ui.add_space(10.0);
        ui.label(RichText::new(title).size(20.0));

        let canvas_response = Frame::canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let size = available_width * vec2(1.0, 0.2);

            let (_id, canvas_rect) = ui.allocate_space(size);

            let max = *data_range.end();
            let min = *data_range.start();
            let data_rect = Rect::from_x_y_ranges(0.0..=GRAPH_HISTORY_SIZE as f32, max..=min);
            let to_screen = RectTransform::from_to(data_rect, canvas_rect);

            let painter = ui.painter().with_clip_rect(canvas_rect);

            graph_content(&painter, to_screen);

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
            let history_index = (graph_pos.x as usize).clamp(0, GRAPH_HISTORY_SIZE - 1);

            popup::show_tooltip(ui.ctx(), Id::new("popup"), |ui| {
                tooltip_content(ui, self.history.get(history_index).unwrap())
            });
        }
    }

    fn draw_latency_graph(&self, ui: &mut Ui, available_width: f32) {
        let mut data = statistics::Data::new(
            self.history
                .iter()
                .map(|stats| stats.total_pipeline_latency_s as f64)
                .collect::<Vec<_>>(),
        );

        self.draw_graph(
            ui,
            available_width,
            "Latency",
            0.0..=(data.quantile(UPPER_QUANTILE) * 1.2) as f32 * 1000.0,
            |painter, to_screen_trans| {
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
                        painter.rect_filled(
                            Rect {
                                min: to_screen_trans * pos2(i as f32, offset + value * 1000.0),
                                max: to_screen_trans * pos2(i as f32 + 2.0, offset),
                            },
                            Rounding::ZERO,
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
        let mut data = statistics::Data::new(
            self.history
                .iter()
                .map(|stats| stats.client_fps)
                .chain(self.history.iter().map(|stats| stats.server_fps))
                .map(|v| v as f64)
                .collect::<Vec<_>>(),
        );
        let upper_quantile = data.quantile(UPPER_QUANTILE);
        let lower_quantile = data.quantile(1.0 - UPPER_QUANTILE);

        let max = upper_quantile + (upper_quantile - lower_quantile);
        let min = lower_quantile - (upper_quantile - lower_quantile);

        self.draw_graph(
            ui,
            available_width,
            "Framerate",
            min as f32..=max as f32,
            |painter, to_screen_trans| {
                let (server_fps_points, client_fps_points) = (0..GRAPH_HISTORY_SIZE)
                    .map(|i| {
                        (
                            to_screen_trans * pos2(i as f32, self.history[i].server_fps),
                            to_screen_trans * pos2(i as f32, self.history[i].client_fps),
                        )
                    })
                    .unzip();

                draw_lines(painter, server_fps_points, graph_colors::SERVER_FPS);
                draw_lines(painter, client_fps_points, graph_colors::CLIENT_FPS);
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
        let mut data = statistics::Data::new(
            self.history
                .iter()
                .map(|stats| stats.actual_bitrate_bps as f64)
                .collect::<Vec<_>>(),
        );

        self.draw_graph(
            ui,
            available_width,
            "Bitrate",
            0.0..=(data.quantile(UPPER_QUANTILE) * 2.0) as f32 / 1e6,
            |painter, to_screen_trans| {
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

                draw_lines(painter, scaled_calculated, Color32::GRAY);
                draw_lines(painter, encoder_latency_limiter, graph_colors::TRANSCODE);
                draw_lines(painter, network_latency_limiter, graph_colors::NETWORK);
                draw_lines(painter, decoder_latency_limiter, graph_colors::TRANSCODE);
                draw_lines(painter, manual_max, graph_colors::RENDER);
                draw_lines(painter, manual_min, graph_colors::RENDER);
                draw_lines(painter, requested, theme::OK_GREEN);
                draw_lines(painter, actual, theme::FG);
            },
            |ui, stats| {
                fn maybe_label(
                    ui: &mut Ui,
                    text: &str,
                    maybe_value_bps: Option<f32>,
                    color: Color32,
                ) {
                    if let Some(value) = maybe_value_bps {
                        ui.colored_label(color, &format!("{text}: {:.2} Mbps", value / 1e6));
                    }
                }

                let n = &stats.nominal_bitrate;

                maybe_label(
                    ui,
                    "Initial calculated",
                    n.scaled_calculated_bps,
                    Color32::GRAY,
                );
                maybe_label(
                    ui,
                    "Encoder latency limiter",
                    n.encoder_latency_limiter_bps,
                    graph_colors::TRANSCODE,
                );
                maybe_label(
                    ui,
                    "Network latency limiter",
                    n.network_latency_limiter_bps,
                    graph_colors::NETWORK,
                );
                maybe_label(
                    ui,
                    "Decoder latency limiter",
                    n.decoder_latency_limiter_bps,
                    graph_colors::TRANSCODE,
                );
                maybe_label(ui, "Manual max", n.manual_max_bps, graph_colors::RENDER);
                maybe_label(ui, "Manual min", n.manual_min_bps, graph_colors::RENDER);
                maybe_label(ui, "Requested", Some(n.requested_bps), theme::OK_GREEN);
                maybe_label(
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
            ui[1].label(&format!("{:.1} Mbps", statistics.video_mbits_per_sec));

            ui[0].label("Total latency:");
            ui[1].label(&format!("{:.0} ms", statistics.total_latency_ms));

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
