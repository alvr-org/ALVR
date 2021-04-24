use super::SettingProps;
use serde_json as json;
use settings_schema::{DictionaryDefault, SchemaDictionary};
use yew::html;
use yew_functional::function_component;

#[function_component(Dictionary)]
pub fn dictionary(props: &SettingProps<SchemaDictionary, DictionaryDefault<json::Value>>) -> Html {
    html!("dictionary")
}
