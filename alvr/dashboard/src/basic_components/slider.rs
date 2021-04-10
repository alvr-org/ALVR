use std::rc::Rc;
use yew::{html, html::InputData, Callback, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub value: String,
    pub default: String,
    pub min: String,
    pub max: String,
    pub step: String,
    pub on_change: Callback<String>,
}

#[function_component(Slider)]
pub fn slider(props: &Props) -> Html {
    let value = props.value.clone();
    let on_change = props.on_change.clone();

    let (value, set_value) = use_state(move || value);

    let on_input = Callback::from(move |data: InputData| set_value(data.value));
    let on_change = {
        let value = Rc::clone(&value);
        Callback::from(move |_| on_change.emit(value.as_ref().clone()))
    };

    let datalist_id = crate::get_id();

    html! {
        <>
            <input
                type="range"
                // class="form-range" -> Bootatrap erases the datalist ticks
                value=*value
                min=props.min
                max=props.max
                step=props.step
                oninput=on_input
                onchange=on_change
                list=datalist_id
            />
            <datalist id=datalist_id>
                // labels not working
                <option value=*value label=*value/>
                <option value=props.min label=props.min/>
                <option value=props.max label=props.max/>
                <option value=props.default label=format!("Default ({})", props.default)/>
            </datalist>
        </>
    }
}
