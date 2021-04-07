use crate::basic_components::Button;
use std::rc::Rc;
use yew::{html, Callback};
use yew_functional::{function_component, use_state};

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let (label, set_label) = use_state(|| "Hello".to_owned());

    let on_click = {
        let label = Rc::clone(&label);
        Callback::from(move |_| set_label(format!("{} world", label)))
    };

    html! {
        <Button onclick=on_click>
            {label}
        </Button>
    }
}
