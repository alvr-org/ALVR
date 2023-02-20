use eframe::egui::{self, Color32, Context, Rounding, Stroke, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(53, 132, 228);
pub const BG: Color32 = Color32::from_rgb(30, 30, 30);
pub const LIGHTER_BG: Color32 = Color32::from_rgb(36, 36, 36);
pub const SECTION_BG: Color32 = Color32::from_rgb(36, 36, 36);
pub const DARKER_BG: Color32 = Color32::from_rgb(26, 26, 26);
pub const SEPARATOR_BG: Color32 = Color32::from_rgb(69, 69, 69);
pub const SELECTED: Color32 = Color32::from_rgb(120, 174, 237);
pub const FG: Color32 = Color32::from_rgb(250, 250, 250);

pub mod log_colors {
    use super::ACCENT;
    use eframe::epaint::Color32;

    pub const ERROR_LIGHT: Color32 = Color32::from_rgb(255, 50, 50);
    pub const WARNING_LIGHT: Color32 = Color32::from_rgb(205, 147, 9);
    pub const INFO_LIGHT: Color32 = ACCENT;
    pub const DEBUG_LIGHT: Color32 = Color32::LIGHT_GRAY;
    pub const EVENT_LIGHT: Color32 = Color32::GRAY;
}

// Graph colors
pub mod graph_colors {
    use eframe::egui::Color32;

    pub const RENDER: Color32 = Color32::RED;
    pub const RENDER_VARIANT: Color32 = Color32::from_rgb(255, 50, 0);
    pub const NETWORK: Color32 = Color32::DARK_GRAY;
    pub const TRANSCODE: Color32 = Color32::LIGHT_BLUE;
    pub const IDLE: Color32 = Color32::YELLOW;
    pub const SERVER_FPS: Color32 = Color32::LIGHT_BLUE;
    pub const CLIENT_FPS: Color32 = Color32::KHAKI;
}

pub fn set_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.slider_width = 200_f32; // slider width can only be set globally
    style.spacing.item_spacing = egui::vec2(15.0, 15.0);
    style.spacing.button_padding = egui::vec2(10.0, 10.0);

    ctx.set_style(style);

    let mut visuals = Visuals::dark();

    let rounding = Rounding::same(10.0);

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.active.rounding = rounding;

    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.inactive.rounding = rounding;

    visuals.widgets.hovered.rounding = rounding;

    visuals.widgets.open.bg_fill = SEPARATOR_BG;
    visuals.widgets.open.rounding = rounding;

    visuals.selection.bg_fill = SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, BG);

    visuals.widgets.noninteractive.bg_fill = BG;
    visuals.faint_bg_color = DARKER_BG;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, SEPARATOR_BG);
    visuals.widgets.noninteractive.rounding = rounding;

    ctx.set_visuals(visuals);
}
