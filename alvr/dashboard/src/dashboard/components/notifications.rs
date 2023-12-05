use alvr_common::{LogEntry, LogSeverity};
use alvr_gui_common::theme::{self, log_colors};
use alvr_session::Settings;
use eframe::{
    egui::{self, Frame, Label, Layout, RichText, TopBottomPanel},
    emath::Align,
    epaint::{Color32, Stroke},
};
use rand::seq::SliceRandom;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

const TIMEOUT: Duration = Duration::from_secs(5);
const NO_NOTIFICATIONS_MESSAGE: &str = "No new notifications";
const NOTIFICATION_TIPS: &[&str] = &[
    // The following tips are ordered roughtly in the order settings appear
    r#"If you started having crashes after changing some settings, reset ALVR by deleting "session.json"."#,
    r#"Some settings are hidden by default. Click the "Expand" button next to some settings to expand the submenus."#,
    r#"It's highly advisable to keep audio setting as default in ALVR and modify the default audio device in the taskbar tray."#,
    r#"Increasing "Maximum buffering" may reduce stutters at the cost of more latency."#,
    r#"Turning off "Optimize game render latency" may improve streaming smoothness."#,
    r#"Sometimes switching between h264 and HEVC codecs is necessary on certain GPUs to fix crashing or fallback to software encoding."#,
    r#"If you're using NVIDIA gpu, best to use high bitrate H264, if you're using AMD gpu, HEVC might look better."#,
    r#"If you experience "white snow" flickering, reduce the resolution to "Low" and disable "Foveated encoding"."#,
    r#"Increasing "Color correction"->"Sharpness" may improve the perceived image quality."#,
    r#"If you have problems syncing external controllers or trackers to ALVR tracking space, add one element to "Extra openvr props", then set a custom "Tracking system name"."#,
    r#"To change the visual appearance of controllers, set "Controllers"->"Emulation mode"."#,
    r#"ALVR supports custom button bindings! If you need help please ask us in the Discord server."#,
    r#"ALVR supports hand tracking gestures. Use thumb-index/middle/ring/pinky to activate different buttons. Joystick is enabled by moving the thumb on a closed fist."#,
    r#"If hand tracking gestures are annoying, you can disable them in "Controllers"->"Gestures". Alternatively you can enable "Gestures"->"Only touch"."#,
    r#"You can fine-tune the controllers responsiveness with "Controllers"->"Prediction"."#,
    r#"If the visual controller/hand models does not match the physical controller, you can tweak the offset in "Controllers"->"Left controller position/rotation offset" (affects both controllers)."#,
    r#"When using external trackers or controllers you should set both "Position/Rotation recentering mode" to "Disabled"."#,
    r#"You can enable tilt mode. Set "Position recentering mode to "Local" and "Rotation recentering mode" to "Tilted"."#,
    r#"If you often experience image glitching, you can trade that with stutter frames using "Avoid video glitching"."#,
    r#"You can run custom commands/programs at client connection/disconnection using "On connect/disconnect script"."#,
    r#"In case you want to report a bug, to get a log file enable "Log to disk". The log will be inside "session_log.txt"."#,
    r#"For hacking purposes, you can enable "Log tracking", "Log button presses", "Log haptics". You can get the data using a websocket at ws://localhost:8082/api/events"#,
    r#"In case you want to report a bug and share your log, you should enable "Prefer backtrace"."#,
    r#"You can quickly cycle through tips like this one by toggling "Show notification tip"."#,
    r#"If you want to use body trackers or other SteamVR drivers together with ALVR, set "Driver launch action" to "Unregister ALVR at shutdown""#,
    r#"It's handy to enable "Open and close SteamVR with dashboard"."#,
    r#"If you want to share a video recording for reporting a bug, you can enable "Rolling video files" to limit the file size of the upload."#,
    // Miscellaneous
    r#"If your headset does not appear in the clients list it might be in a different subnet. Try "Add client manually"."#,
    r#"For audio setup on Linux, check the wiki at https://github.com/alvr-org/ALVR/wiki/Installation-guide#automatic-audio--microphone-setup"#,
    r#"ALVR supports wired connection using USB. Check the wiki at https://github.com/alvr-org/ALVR/wiki/ALVR-wired-setup-(ALVR-over-USB)"#,
    r#"You can record a video of the gameplay using "Start recording" in the "Debug" category in the sidebar."#,
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
        self.min_notification_level = settings.logging.notification_level;

        if settings.logging.show_notification_tip {
            if self.tip_message.is_none() {
                self.tip_message = NOTIFICATION_TIPS
                    .choose(&mut rand::thread_rng())
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

    pub fn ui(&mut self, context: &egui::Context) {
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

        let mut bottom_bar = TopBottomPanel::bottom("bottom_panel").frame(
            Frame::default()
                .inner_margin(egui::vec2(10.0, 5.0))
                .fill(bg)
                .stroke(Stroke::new(1.0, theme::SEPARATOR_BG)),
        );
        let alignment = if !self.expanded {
            bottom_bar = bottom_bar.max_height(26.0);

            Align::TOP
        } else {
            Align::Center
        };

        bottom_bar.show(context, |ui| {
            ui.with_layout(Layout::right_to_left(alignment), |ui| {
                if !self.expanded {
                    if ui.small_button("Expand").clicked() {
                        self.expanded = true;
                    }
                } else if ui.button("Reduce").clicked() {
                    self.expanded = false;
                }
                ui.with_layout(Layout::left_to_right(alignment), |ui| {
                    ui.add(
                        Label::new(RichText::new(&self.message).color(fg).size(12.0)).wrap(true),
                    );
                })
            })
        });
    }
}
