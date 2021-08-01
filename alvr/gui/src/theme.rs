use alvr_common::data::Theme;
use egui::{CtxRef, Visuals};

pub fn set_theme(ctx: &CtxRef, theme: Theme) {
    let mut style = (*ctx.style()).clone();
    style.spacing.slider_width = 200_f32; // slider width can only be set globally
    ctx.set_style(style);

    let visuals = match theme {
        Theme::SystemDefault => {
            #[cfg(any(windows, target_os = "macos"))]
            match dark_light::detect() {
                dark_light::Mode::Dark => Visuals::dark(),
                dark_light::Mode::Light => Visuals::light(),
            }
            #[cfg(not(any(windows, target_os = "macos")))]
            Visuals::dark()
        }
        Theme::Classic => Visuals::light(),
        Theme::Darkly => Visuals::dark(),
    };

    ctx.set_visuals(visuals);
}
