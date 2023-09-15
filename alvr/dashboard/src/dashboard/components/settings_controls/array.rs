use super::{collapsible, NestingInfo, SettingControl};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::SchemaNode;
use eframe::egui::Ui;
use serde_json as json;

pub struct Control {
    nesting_info: NestingInfo,
    controls: Vec<SettingControl>,
}

impl Control {
    pub fn new(nesting_info: NestingInfo, schema_array: Vec<SchemaNode>) -> Self {
        let controls = schema_array
            .into_iter()
            .enumerate()
            .map(|(idx, schema)| {
                let mut nesting_info = nesting_info.clone();
                nesting_info.path.push("content".into());
                nesting_info.path.push(idx.into());

                SettingControl::new(nesting_info, schema)
            })
            .collect();

        Self {
            nesting_info,
            controls,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        super::grid_flow_inline(ui, allow_inline);

        let mut request = None;

        let collapsed =
            collapsible::collapsible_button(ui, &self.nesting_info, session_fragment, &mut request);

        if !collapsed {
            let session_array_mut = session_fragment["content"].as_array_mut().unwrap();

            for (idx, control) in self.controls.iter_mut().enumerate() {
                ui.end_row();

                request = control
                    .ui(ui, &mut session_array_mut[idx], false)
                    .or(request);
            }
        }

        request
    }
}
