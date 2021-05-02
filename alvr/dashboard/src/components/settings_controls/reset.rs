use crate::basic_components::IconButton;
use alvr_common::data::SessionDesc;
use std::fmt::Display;
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_context, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props<T: Clone + PartialEq> {
    pub default: T,
    pub show_prompt: bool,
    pub set_value: Callback<T>,
}

#[function_component(Reset)]
pub fn reset<T: Display + Clone + PartialEq>(props: &Props<T>) -> Html {
    let should_show_modal = use_context::<SessionDesc>()
        .unwrap()
        .session_settings
        .extra
        .revert_confirm_dialog;

    let (modal_visible, set_modal_visible) = use_state(|| false);

    html! {
        <>
            <IconButton
                icon_cls="fas fa-undo"
            />
            {
                if *modal_visible {
                    html!{}
                } else {
                    html!()
                }
            }
        </>
    }
}
