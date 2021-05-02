use super::SettingProps;
use yew::html;
use yew_functional::function_component;

#[function_component(Boolean)]
pub fn boolean(props: &SettingProps<bool, bool>) -> Html {
    html!("boolean")
}
