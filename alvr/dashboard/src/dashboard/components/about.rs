use crate::dashboard::DashboardResponse;
use alvr_session::SessionDesc;
use egui::{RichText, Ui};

pub struct AboutTab {}

impl AboutTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui, session: &SessionDesc) -> Option<DashboardResponse> {
        ui.label(RichText::new(&format!("ALVR server {}", session.server_version)).size(30.0));
        ui.label(
r#"Stream VR games from your PC to your headset via Wi-Fi.
ALVR uses technologies like Asynchronous TimeWarp (ATW) and Fixed Foveated Rendering (FFR) for a smoother experience.
All games that work with an Oculus Rift(s) should work with ALVR.
This is a fork of ALVR that works with Oculus Quest and Go.
"#
        );
        ui.add_space(10.0);
        ui.hyperlink_to("Visit us on GitHub", "https://github.com/alvr-org/ALVR");
        ui.hyperlink_to("Join us on Discord", "https://discord.gg/ALVR");
        ui.hyperlink_to(
            "Latest release",
            "https://github.com/alvr-org/ALVR/releases/latest",
        );

        None
    }
}
