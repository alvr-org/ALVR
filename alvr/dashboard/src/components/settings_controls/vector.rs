use super::SettingProps;
use serde_json as json;
use settings_schema::{SchemaVector, VectorDefault};
use yew::html;
use yew_functional::function_component;

#[function_component(Vector)]
pub fn vector(props: &SettingProps<SchemaVector, VectorDefault<json::Value>>) -> Html {
    html!("dictionary")
}
