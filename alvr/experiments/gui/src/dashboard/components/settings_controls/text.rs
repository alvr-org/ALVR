use super::{SettingControl, SettingsContext, SettingsResponse};
use egui::{TextEdit, Ui};
use serde_json as json;

pub struct Text {
    value: String,
    default: String,
}

impl Text {
    pub fn new(default: String, session_fragment: json::Value) -> Self {
        Self {
            default,
            value: json::from_value(session_fragment).unwrap(),
        }
    }
}

impl SettingControl for Text {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        ctx: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let textbox = TextEdit::singleline(&mut self.value).desired_width(50_f32);
        let res = ui.add(textbox);

        let response = if res.lost_focus() {
            Some(super::into_fragment(&self.value))
        } else {
            if !res.has_focus() {
                self.value = json::from_value(session_fragment).unwrap();
            }

            None
        };

        super::reset_clicked(
            ui,
            &self.value,
            &self.default,
            &format!("\"{}\"", self.default),
            &ctx.t,
        )
        .then(|| super::into_fragment(&self.default))
        .or(response)
    }
}
