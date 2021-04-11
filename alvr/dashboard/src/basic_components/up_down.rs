use std::rc::Rc;
use yew::{html, Callback, InputData, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub value: String,

    pub on_focus_lost: Callback<String>,
    pub on_step_down: Callback<()>,
    pub on_step_up: Callback<()>,
}

#[function_component(UpDown)]
pub fn up_down(props: &Props) -> Html {
    let value = props.value.clone();
    let on_focus_lost = props.on_focus_lost.clone();
    let on_step_down = props.on_step_down.clone();
    let on_step_up = props.on_step_up.clone();

    let (value, set_value) = use_state(|| value);

    let on_input = Callback::from(move |data: InputData| set_value(data.value));

    let on_focus_lost = {
        let value = Rc::clone(&value);
        Callback::from(move |_| on_focus_lost.emit(value.as_ref().clone()))
    };

    html! {
        <div class="input-group">
            <button
                class="btn btn-outline-primary btn-sm"
                onclick=Callback::from(move |_| on_step_down.emit(()))
            >
                <i class="fa fa-minus" />
            </button>
            <input
                type="text"
                value=*value
                oninput=on_input
                onblur=on_focus_lost
            />
            <button
                class="btn btn-outline-primary btn-sm"
                onclick=Callback::from(move |_| on_step_up.emit(()))
            >
                <i class="fa fa-plus" />
            </button>
        </div>
    }
}
