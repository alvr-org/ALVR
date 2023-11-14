use alvr_common::ALVR_VERSION;
use eframe::egui::{RichText, Ui};

pub fn about_tab_ui(ui: &mut Ui) {
    ui.label(RichText::new(format!("ALVR streamer v{}", *ALVR_VERSION)).size(30.0));
    ui.label(
r#"Stream VR games from your PC to your headset via Wi-Fi.
ALVR uses technologies like Asynchronous TimeWarp (ATW) and Fixed Foveated Rendering (FFR) for a smoother experience.
All games that work with an Oculus Rift(s) should work with ALVR.
This is a fork of ALVR that works with Meta Quest and other standalone headsets.
"#
    );
    ui.add_space(10.0);
    ui.hyperlink_to("Visit us on GitHub", "https://github.com/alvr-org/ALVR");
    ui.hyperlink_to("Join us on Discord", "https://discord.gg/ALVR");
    ui.hyperlink_to(
        "Latest release",
        "https://github.com/alvr-org/ALVR/releases/latest",
    );
    ui.hyperlink_to(
        "Donate to ALVR on Open Collective",
        "https://opencollective.com/alvr",
    );
}
