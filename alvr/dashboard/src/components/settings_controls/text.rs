use super::SettingProps;
use yew::html;
use yew_functional::function_component;

#[function_component(Text)]
pub fn text(props: &SettingProps<String, String>) -> Html {
    html!("integer")
}
