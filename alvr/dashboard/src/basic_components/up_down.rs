use std::rc::Rc;
use yew::{html, Callback, InputData, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub value: String,

    #[prop_or_default]
    pub label: String,

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
        <div>
            {
                if props.label.len() != 0 {
                    html! {
                        <label class="block text-sm text-gray-700 font-medium">
                            {props.label.clone()}
                        </label>
                    }
                } else {
                    html! {}
                }
            }
            <div class="flex shadow-sm">
                <button
                    class="rounded-l border text-gray-500 hover:bg-gray-200 p-1 w-8"
                    onclick=Callback::from(move |_| on_step_down.emit(()))
                >
                    <i class="fa fa-minus" />
                </button>
                // todo: adapt size to content
                <input
                    class="border-t border-b  px-2 py-1 flex-1"
                    type="text"
                    value=*value
                    oninput=on_input
                    onblur=on_focus_lost
                />
                <button
                    class="rounded-r border text-gray-500 hover:bg-gray-200 p-1 w-8"
                    onclick=Callback::from(move |_| on_step_up.emit(()))
                >
                    <i class="fa fa-plus" />
                </button>
            </div>
        </div>
    }
}
