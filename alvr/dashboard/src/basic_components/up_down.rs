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
    let value_handle = {
        let value = props.value.clone();
        use_state(|| value)
    };

    let on_input = {
        let value_handle = value_handle.clone();
        Callback::from(move |data: InputData| value_handle.set(data.value))
    };

    let on_focus_lost = {
        let on_focus_lost = props.on_focus_lost.clone();
        let value_handle = value_handle.clone();
        Callback::from(move |_| on_focus_lost.emit((*value_handle).clone()))
    };

    let on_step_down = {
        let on_step_down = props.on_step_down.clone();
        Callback::from(move |_| on_step_down.emit(()))
    };

    let on_step_up = {
        let on_step_up = props.on_step_up.clone();
        Callback::from(move |_| on_step_up.emit(()))
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
            <div class="flex shadow-sm">
                <button
                    class="rounded-l border text-gray-500 hover:bg-gray-200 p-1 w-8"
                    onclick=on_step_down
                >
                    <i class="fas fa-minus" />
                </button>
                // todo: adapt size to content
                <input
                    class="border-t border-b  px-2 py-1 flex-1"
                    type="text"
                    value=*value_handle
                    oninput=on_input
                    onblur=on_focus_lost
                />
                <button
                    class="rounded-r border text-gray-500 hover:bg-gray-200 p-1 w-8"
                    onclick=on_step_up
                >
                    <i class="fas fa-plus" />
                </button>
            </div>
        </div>
    }
}
