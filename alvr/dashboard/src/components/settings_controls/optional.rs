use super::SettingProps;
use serde_json as json;
use settings_schema::{OptionalDefault, SchemaOptional};
use yew::html;
use yew_functional::function_component;

#[function_component(OptionalControl)]
pub fn optional_control(
    props: &SettingProps<SchemaOptional, OptionalDefault<json::Value>>,
) -> Html {
    html!("optional control")
}

#[function_component(OptionalContainer)]
pub fn optional_container(
    props: &SettingProps<SchemaOptional, OptionalDefault<json::Value>>,
) -> Html {
    html!("optional container")
}
