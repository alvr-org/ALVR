use crate::{
    basic_components::{IconButton, Modal, RawHtml},
    session,
    translation::use_translation,
};
use alvr_common::data::SessionDesc;
use std::{fmt::Display, rc::Rc};
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_context, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props<T: Clone + PartialEq> {
    pub default: T,
    pub set_default: Callback<T>,
}

#[function_component(Reset)]
pub fn reset<T: Display + Clone + PartialEq + 'static>(props: &Props<T>) -> Html {
    let t = use_translation();

    let should_show_modal = use_context::<SessionDesc>()
        .unwrap()
        .session_settings
        .extra
        .revert_confirm_dialog;

    let (modal_visible, set_modal_visible) = use_state(|| false);

    let on_reset_requested = {
        let set_modal_visible = Rc::clone(&set_modal_visible);
        Callback::from(move |_| set_modal_visible(should_show_modal))
    };

    let on_ok = {
        let default = props.default.clone();
        let set_default = props.set_default.clone();
        let set_modal_visible = Rc::clone(&set_modal_visible);
        Callback::from(move |do_not_ask_again: bool| {
            // Use partial session to trigger extrapolation. This avoids race-conditions between
            // requests (synce they are async).
            let partial_session_settings = serde_json::json!({
                "extra": {
                    "revert_confirm_dialog": !do_not_ask_again
                }
            });

            crate::spawn_str_result_future(async move {
                session::apply_session_settings_raw(&partial_session_settings).await
            });

            set_default.emit(default.clone());

            set_modal_visible(false);
        })
    };

    let icon_html = html! {
        <IconButton
            icon_cls="fas fa-undo"
            on_click=on_reset_requested
        />
    };

    if *modal_visible {
        let args = fluent::fluent_args! {
            "value" => format!("<strong>{}</strong>", props.default)
        };

        html! {
            <>
                {icon_html}
                <Modal
                    use_do_not_ask_again=true
                    on_ok=on_ok
                    on_cancel=Callback::from(move |_| set_modal_visible(false))
                >
                    <RawHtml html=t.with_args("reset-prompt", &args).as_ref() />
                </Modal>
            </>
        }
    } else {
        icon_html
    }
}
