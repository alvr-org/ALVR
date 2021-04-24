use super::SettingProps;
use serde_json as json;
use settings_schema::EntryType;
use std::collections::HashMap;
use yew::html;
use yew_functional::function_component;

#[function_component(Section)]
pub fn section(
    props: &SettingProps<Vec<(String, EntryType)>, HashMap<String, json::Value>>,
) -> Html {
    html!("section")
}
