use std::rc::Rc;
use yew::{html, Callback, InputData, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub value: String,

    #[prop_or_default]
    pub placeholder: String,

    #[prop_or_default]
    pub label: String,

    pub on_focus_lost: Callback<String>,
}

#[function_component(TextField)]
pub fn text_field(props: &Props) -> Html {
    let value = props.value.clone();
    let on_focus_lost = props.on_focus_lost.clone();

    let (value, set_value) = use_state(|| value);

    let on_input = Callback::from(move |data: InputData| set_value(data.value));

    let on_focus_lost = {
        let value = Rc::clone(&value);
        Callback::from(move |_| on_focus_lost.emit(value.as_ref().clone()))
    };

    html! {
        <div>
            {
                if !props.label.is_empty() {
                    html! {
                        <label class="block text-sm text-gray-700 font-medium">
                            {props.label.clone()}
                        </label>
                    }
                } else {
                    html! {}
                }
            }
            // todo: adapt size to content
            <input
                class="rounded border border-gray-300 px-2 py-1 shadow-sm"
                type="text"
                value=*value
                placeholder=props.placeholder
                oninput=on_input
                onblur=on_focus_lost
            />
        </div>
    }
}
