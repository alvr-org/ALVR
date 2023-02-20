use super::{NestingInfo, SettingControl};
use alvr_session::settings_schema::SchemaNode;
use alvr_sockets::DashboardRequest;
use eframe::egui::Ui;
use serde_json as json;

pub struct Control {
    controls: Vec<SettingControl>,
}

impl Control {
    pub fn new(nesting_info: NestingInfo, schema_array: Vec<SchemaNode>) -> Self {
        let controls = schema_array
            .into_iter()
            .enumerate()
            .map(|(idx, schema)| {
                let mut nesting_info = nesting_info.clone();
                nesting_info.path.push(idx.into());

                SettingControl::new(nesting_info, schema)
            })
            .collect();

        Self { controls }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<DashboardRequest> {
        super::grid_flow_inline(ui, allow_inline);

        let session_array_mut = session_fragment.as_array_mut().unwrap();

        let count = self.controls.len();

        let mut request = None;
        for (idx, control) in self.controls.iter_mut().enumerate() {
            let allow_inline = idx == 0;
            request = control
                .ui(ui, &mut session_array_mut[idx], allow_inline)
                .or(request);

            if idx != count - 1 {
                ui.end_row();
            }
        }

        request
    }
}
