use yew::{html, Callback};
use yew_functional::{function_component, use_state};

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let (label, set_label) = use_state(|| "Hello".to_owned());

    let on_click = {
        let label = label.clone();
        Callback::from(move |_| set_label(format!("{} world", label)))
    };

    html! {
        <button class="btn btn-primary" onclick=on_click>
            {label}
        </button>
    }
}
