use super::{Reset, SettingProps};
use crate::basic_components::Switch;
use yew::html;
use yew_functional::function_component;

#[function_component(Boolean)]
pub fn boolean(props: &SettingProps<bool, bool>) -> Html {
    html! {
        <>
            <Switch checked=props.session on_click=props.set_session.clone() />
            <Reset<bool> default=props.schema set_default=props.set_session.clone() />
        </>
    }
}
