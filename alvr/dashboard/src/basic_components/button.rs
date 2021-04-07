use web_sys::MouseEvent;
use yew::{html, Callback, Children, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub children: Children,
    pub onclick: Callback<MouseEvent>,
}

// note: this is an example, buttons should not need a Yew component, tailwind-css classes should be grouped into custom
// classes and the imported where needed. This way we don't need to forward children and events
#[function_component(Button)]
pub fn button(Props { children, onclick }: &Props) -> Html {
    html! {
        <button
            class="relative bg-blue-500 text-white p-6 rounded text-2xl font-bold overflow-visible"
            onclick=onclick
        >
            {children.clone()}
        </button>
    }
}
