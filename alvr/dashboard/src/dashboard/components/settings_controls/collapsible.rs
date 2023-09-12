use super::NestingInfo;
use alvr_packets::PathValuePair;
use eframe::egui::Ui;
use serde_json as json;

pub fn collapsible_button(
    ui: &mut Ui,
    nesting_info: &NestingInfo,
    session_fragment: &mut json::Value,
    request: &mut Option<PathValuePair>,
) -> bool {
    let json::Value::Bool(state_mut) = &mut session_fragment["gui_collapsed"] else {
        unreachable!()
    };

    if (*state_mut && ui.small_button("Expand").clicked())
        || (!*state_mut && ui.small_button("Collapse").clicked())
    {
        *state_mut = !*state_mut;
        *request = super::get_single_value(
            nesting_info,
            "gui_collapsed".into(),
            json::Value::Bool(*state_mut),
        );
    }

    *state_mut
}
