use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub checked: bool,
    pub on_click: Callback<bool>,
}

#[function_component(Switch)]
pub fn switch(props: &Props) -> Html {
    let Props { checked, on_click } = props.clone();

    let checked = use_state(move || checked);

    let on_click = {
        let checked = checked.clone();
        Callback::from(move |_| {
            on_click.emit(!*checked);
            checked.set(!*checked);
        })
    };

    html! {
        <div class="form-check form-switch">
            <input
                class="form-check-input"
                type="checkbox"
                onclick=on_click
                checked=*checked
            />
        </div>
    }
}
