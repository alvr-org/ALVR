use web_sys::MouseEvent;
use yew::{html, Callback, Children, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub children: Children,
    pub onclick: Callback<MouseEvent>,
}

#[function_component(Button)]
pub fn button(Props { children, onclick }: &Props) -> Html {
    html! {
        <button class="btn btn-primary" onclick=onclick>
            {children.clone()}
        </button>
    }
}
