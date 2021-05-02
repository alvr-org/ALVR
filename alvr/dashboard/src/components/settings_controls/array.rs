use super::SettingProps;
use serde_json as json;
use settings_schema::SchemaNode;
use yew::html;
use yew_functional::function_component;

#[function_component(Array)]
pub fn array(props: &SettingProps<Vec<SchemaNode>, Vec<json::Value>>) -> Html {
    html!("array")
}
