use yew::{html, Callback, Children, Properties};
use yew_functional::function_component;

#[derive(Clone, PartialEq)]
pub enum ButtonType {
    Primary,
    Secondary,
    Danger,
    None,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub children: Children,
    pub on_click: Callback<()>,
    pub button_type: ButtonType,
}

#[function_component(Button)]
pub fn button(props: &Props) -> Html {
    let on_click = props.on_click.clone();

    let class_type = match props.button_type {
        ButtonType::Primary => "btn-primary",
        ButtonType::Secondary => "btn-secondary",
        ButtonType::Danger => "btn-danger",
        ButtonType::None => "",
    };

    html! {
        <button
            class=format!("btn {}", class_type)
            onclick=Callback::from(move |_| on_click.emit(()))
        >
            {props.children.clone()}
        </button>
    }
}
