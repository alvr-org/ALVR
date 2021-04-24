use super::SettingProps;
use serde_json as json;
use settings_schema::{SchemaSwitch, SwitchDefault};
use yew::html;
use yew_functional::function_component;

#[function_component(SwitchControl)]
pub fn switch_control(props: &SettingProps<SchemaSwitch, SwitchDefault<json::Value>>) -> Html {
    html!("switch control")
}

#[function_component(SwitchContainer)]
pub fn switch_container(props: &SettingProps<SchemaSwitch, SwitchDefault<json::Value>>) -> Html {
    html!("switch container")
}
