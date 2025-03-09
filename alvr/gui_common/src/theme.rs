use egui::{self, Color32, Context, CornerRadius, Stroke, TextStyle, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(0, 76, 176);
pub const BG: Color32 = Color32::from_rgb(30, 30, 30);
pub const LIGHTER_BG: Color32 = Color32::from_rgb(36, 36, 36);
pub const SECTION_BG: Color32 = Color32::from_rgb(36, 36, 36);
pub const DARKER_BG: Color32 = Color32::from_rgb(26, 26, 26);
pub const SEPARATOR_BG: Color32 = Color32::from_rgb(69, 69, 69);
pub const FG: Color32 = Color32::from_rgb(250, 250, 250);

pub const OK_GREEN: Color32 = Color32::GREEN;
pub const KO_RED: Color32 = Color32::RED;

pub mod log_colors {
    use egui::epaint::Color32;

    pub const ERROR_LIGHT: Color32 = Color32::from_rgb(255, 50, 50);
    pub const WARNING_LIGHT: Color32 = Color32::from_rgb(205, 147, 9);
    pub const INFO_LIGHT: Color32 = Color32::from_rgb(134, 171, 241);
    pub const DEBUG_LIGHT: Color32 = Color32::LIGHT_GRAY;
    pub const EVENT_LIGHT: Color32 = Color32::GRAY;
}

// Graph colors
pub mod graph_colors {
    use egui::Color32;

    // Colors taken from https://colorhunt.co/palette/ff6b6bffd93d6bcb774d96ff
    pub const RENDER_EXTERNAL: Color32 = Color32::from_rgb(64, 64, 64);
    pub const RENDER_EXTERNAL_LABEL: Color32 = Color32::GRAY;
    pub const RENDER: Color32 = Color32::RED;
    pub const IDLE: Color32 = Color32::from_rgb(255, 217, 61);
    pub const TRANSCODE: Color32 = Color32::from_rgb(107, 203, 119);
    pub const NETWORK: Color32 = Color32::from_rgb(77, 150, 255);

    pub const SERVER_FPS: Color32 = Color32::LIGHT_BLUE;
    pub const CLIENT_FPS: Color32 = Color32::KHAKI;

    pub const INITIAL_CALCULATED_THROUGHPUT: Color32 = Color32::GRAY;
    pub const ENCODER_DECODER_LATENCY_LIMITER: Color32 = TRANSCODE;
    pub const NETWORK_LATENCY_LIMITER: Color32 = NETWORK;
    pub const MIN_MAX_LATENCY_THROUGHPUT: Color32 = Color32::RED;
    pub const REQUESTED_BITRATE: Color32 = Color32::GREEN;
    pub const RECORDED_THROUGHPUT: Color32 = Color32::KHAKI;
    pub const RECORDED_BITRATE: Color32 = super::FG;
}

pub fn set_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.slider_width = 200_f32; // slider width can only be set globally
    style.spacing.item_spacing = egui::vec2(15.0, 15.0);
    style.spacing.button_padding = egui::vec2(10.0, 10.0);
    style.text_styles.get_mut(&TextStyle::Body).unwrap().size = 14.0;

    ctx.set_style(style);

    let mut visuals = Visuals::dark();

    let corner_radius = CornerRadius::same(10);

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.active.corner_radius = corner_radius;

    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.inactive.corner_radius = corner_radius;

    visuals.widgets.hovered.corner_radius = corner_radius;

    visuals.widgets.open.bg_fill = SEPARATOR_BG;
    visuals.widgets.open.corner_radius = corner_radius;

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, FG);

    visuals.widgets.noninteractive.bg_fill = BG;
    visuals.faint_bg_color = DARKER_BG;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, SEPARATOR_BG);
    visuals.widgets.noninteractive.corner_radius = corner_radius;

    ctx.set_visuals(visuals);
}
