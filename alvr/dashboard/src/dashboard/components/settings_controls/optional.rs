use super::{reset, NestingInfo, SettingControl};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::SchemaNode;
use eframe::{
    egui::{Layout, Ui},
    emath::Align,
};
use serde_json as json;

pub struct Control {
    nesting_info: NestingInfo,
    default_set: bool,
    default_string: String,
    content_control: Box<SettingControl>,
}

impl Control {
    pub fn new(nesting_info: NestingInfo, default_set: bool, schema_content: SchemaNode) -> Self {
        let default_string = if default_set {
            "Set".into()
        } else {
            "Default".into()
        };

        let control = {
            let mut nesting_info = nesting_info.clone();
            nesting_info.path.push("content".into());

            SettingControl::new(nesting_info, schema_content)
        };

        Self {
            nesting_info,
            default_set,
            default_string,
            content_control: Box::new(control),
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        super::grid_flow_inline(ui, allow_inline);

        let session_switch_mut = session_fragment.as_object_mut().unwrap();

        let json::Value::Bool(set_mut) = &mut session_switch_mut["set"] else {
            unreachable!()
        };

        let mut request = None;

        fn get_request(nesting_info: &NestingInfo, enabled: bool) -> Option<PathValuePair> {
            super::get_single_value(nesting_info, "set".into(), json::Value::Bool(enabled))
        }

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            if ui.selectable_value(set_mut, false, "Default").clicked()
                || ui.selectable_value(set_mut, true, "Set").clicked()
            {
                request = get_request(&self.nesting_info, *set_mut);
            }

            if reset::reset_button(ui, *set_mut != self.default_set, &self.default_string).clicked()
            {
                request = get_request(&self.nesting_info, self.default_set);
            }
        });

        if *set_mut {
            ui.end_row();

            request = self
                .content_control
                .ui(ui, &mut session_switch_mut["content"], false)
                .or(request);
        }

        request
    }
}
