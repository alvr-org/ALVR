use alvr_common::{LogEntry, LogSeverity};
use alvr_gui_common::theme::{self, log_colors};
use alvr_session::Settings;
use eframe::{
    egui::{self, Frame, Label, Layout, Panel, RichText, TextWrapMode, Ui},
    emath::Align,
    epaint::Color32,
};
use rand::seq::IndexedRandom;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

const TIMEOUT: Duration = Duration::from_secs(5);
const NO_NOTIFICATIONS_MESSAGE: &str = "No new notifications";
const NOTIFICATION_TIPS: &[&str] = &[
    // The following tips are ordered roughtly in the order settings appear
    r#"If you started having crashes after changing some settings, reset ALVR by re-running "Run setup wizard" from the "Installation" tab and clicking "Reset settings"."#,
    r#"Some settings are hidden by default. Click the "Expand" button next to some settings to expand the submenus."#,
    r#"It's highly advisable to keep audio settings as default in ALVR and modify the default audio device in the taskbar tray."#,
    r#"Increasing "Video"->"Maximum buffering" may reduce stutters at the cost of more latency."#,
    r#"Sometimes switching between h264 and HEVC codecs is necessary on certain GPUs to fix crashing or fallback to software encoding."#,
    r#"If you're using an NVIDIA GPU, it's best to use high-bitrate H264; if you're using an AMD GPU, HEVC might look better."#,
    r#"If you experience "white snow" flickering, set "Presets"->"Resolution" to "Low" and disable "Video"->"Foveated encoding"."#,
    r#"Increasing "Video"->"Color correction"->"Sharpness" may improve the perceived image quality."#,
    r#"If you have problems syncing external controllers or trackers to ALVR tracking space, add one element to "Headset"->"Extra OpenVR properties", then set a custom "Tracking system name string"."#,
    r#"To change the visual appearance of controllers, set "Headset"->"Controllers"->"Emulation mode"."#,
    r#"ALVR supports custom button bindings! If you need help, please ask us on our Discord server."#,
    r#"ALVR supports hand tracking gestures ("Presets"->"Hand tracking interaction"->"ALVR bindings"). Check out wiki how to use them properly: https://github.com/alvr-org/ALVR/wiki/Hand-tracking-controller-bindings."#,
    r#"If hand tracking gestures are annoying, you can disable them in "Headset"->"Controllers"->"Hand tracking interaction". Alternatively, you can enable "Hand tracking interaction"->"Only touch"."#,
    r#"You can fine-tune the controllers' responsiveness with "Headset"->"Controllers"->"Prediction"."#,
    r#"If the visual controller/hand models do not match the physical controller's position, you can tweak the offset in "Headset"->"Controllers"->"Left controller position/rotation offset" (affects both controllers)."#,
    r#"When using external trackers or controllers, you should set both "Headset"->"Position/Rotation recentering mode" to "Disabled"."#,
    r#"You can enable tilt mode. Set "Headset"->"Position recentering mode" to "Local" and "Headset"->"Rotation recentering mode" to "Tilted"."#,
    r#"If you often experience image glitching, you can trade that with stutter frames using "Connection"->"Avoid video glitching"."#,
    r#"You can run custom commands/programs at headset connection/disconnection using "Connection"->"Enable on connect/disconnect script"."#,
    r#"In case you want to report a bug, to get a log file, enable "Extra"->"Logging"->"Log to disk". The log will be inside "session_log.txt"."#,
    r#"For hacking purposes, you can enable "Extra"->"Logging"->"Log tracking", "Log button presses" and "Log haptics". You can get the data using a websocket at ws://localhost:8082/api/events."#,
    r#"In case you want to report a bug and share your log, you should enable "Extra"->"Logging"->"Prefer backtrace"."#,
    r#"You can quickly cycle through tips like this one by toggling "Extra"->"Logging"->"Show notification tip"."#,
    r#"It's handy to enable "Extra"->"SteamVR Launcher"->"Open and close SteamVR automatically"."#,
    r#"If you want to share a video recording for reporting a bug, you can enable "Extra"->"Capture"->"Rolling video files" to limit the file size of the upload."#,
    // Miscellaneous
    r#"If your headset does not appear in the device list, it might be in a different subnet. Try "Add device manually" with IP shown from inside device."#,
];

pub struct NotificationBar {
    message: String,
    current_level: LogSeverity,
    receive_instant: Instant,
    min_notification_level: LogSeverity,
    tip_message: Option<String>,
    expanded: bool,
}

impl NotificationBar {
    pub fn new() -> Self {
        Self {
            message: NO_NOTIFICATIONS_MESSAGE.into(),
            current_level: LogSeverity::Debug,
            receive_instant: Instant::now(),
            min_notification_level: LogSeverity::Debug,
            tip_message: None,
            expanded: false,
        }
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.min_notification_level = settings.extra.logging.notification_level;

        if settings.extra.logging.show_notification_tip {
            if self.tip_message.is_none() {
                self.tip_message = NOTIFICATION_TIPS
                    .choose(&mut rand::rng())
                    .map(|s| format!("Tip: {s}"));
            }
        } else {
            self.tip_message = None;
        }
    }

    pub fn push_notification(&mut self, event: LogEntry, from_dashboard: bool) {
        let now = Instant::now();
        let min_severity = if from_dashboard {
            if cfg!(debug_assertions) {
                LogSeverity::Debug
            } else {
                LogSeverity::Info
            }
        } else {
            self.min_notification_level
        };

        if event.severity >= min_severity
            && (now > self.receive_instant + TIMEOUT || event.severity >= self.current_level)
        {
            self.message = event.content;
            self.current_level = event.severity;
            self.receive_instant = now;
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let now = Instant::now();
        if now > self.receive_instant + TIMEOUT {
            self.message = self
                .tip_message
                .clone()
                .unwrap_or_else(|| NO_NOTIFICATIONS_MESSAGE.into());
            self.current_level = LogSeverity::Debug;
        }

        let (fg, bg) = match self.current_level {
            LogSeverity::Error => (Color32::BLACK, log_colors::ERROR_LIGHT),
            LogSeverity::Warning => (Color32::BLACK, log_colors::WARNING_LIGHT),
            LogSeverity::Info => (Color32::BLACK, log_colors::INFO_LIGHT),
            LogSeverity::Debug => (theme::FG, theme::LIGHTER_BG),
        };

        let mut bottom_bar = Panel::bottom("bottom_panel").frame(
            Frame::default()
                .inner_margin(egui::vec2(10.0, 5.0))
                .fill(bg),
        );
        let alignment = if !self.expanded {
            bottom_bar = bottom_bar.max_size(26.0);

            Align::TOP
        } else {
            Align::Center
        };
        let wrapping = if !self.expanded {
            TextWrapMode::Truncate
        } else {
            TextWrapMode::Wrap
        };

        bottom_bar.show_inside(ui, |ui| {
            ui.with_layout(Layout::right_to_left(alignment), |ui| {
                if !self.expanded {
                    if ui.small_button("Expand").clicked() {
                        self.expanded = true;
                    }
                } else if ui.button("Reduce").clicked() {
                    self.expanded = false;
                }
                ui.with_layout(Layout::left_to_right(alignment), |ui| {
                    //A LayoutJob that has its TextWrapping updated to fill the available space would probably be a more elegant solution.
                    ui.add(
                        Label::new(RichText::new(&self.message).color(fg).size(12.0))
                            .wrap_mode(wrapping),
                    );
                })
            })
        });
    }
}
