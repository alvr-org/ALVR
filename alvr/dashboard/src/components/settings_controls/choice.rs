use super::SettingProps;
use serde_json as json;
use settings_schema::SchemaChoice;
use std::collections::HashMap;
use yew::html;
use yew_functional::function_component;

#[function_component(ChoiceControl)]
pub fn choice_control(props: &SettingProps<SchemaChoice, HashMap<String, json::Value>>) -> Html {
    html!("choice control")
}

#[function_component(ChoiceContainer)]
pub fn choice_container(props: &SettingProps<SchemaChoice, HashMap<String, json::Value>>) -> Html {
    html!("choice container")
}
