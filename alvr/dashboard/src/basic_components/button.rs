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

    #[prop_or_default]
    pub class: String,

    #[prop_or(ButtonType::Primary)]
    pub button_type: ButtonType,
}

#[function_component(Button)]
pub fn button(props: &Props) -> Html {
    let on_click = props.on_click.clone();

    // TODO: if we add a disabled prop, we need to disable the background color hover changes
    let class_type = match props.button_type {
        ButtonType::Primary => " bg-blue-500 text-blue-50 hover:bg-blue-600",
        ButtonType::Secondary => "border text-gray-800 hover:bg-gray-200",
        ButtonType::Danger => "bg-red-500 text-red-50 hover:bg-red-600",
        ButtonType::None => "text-gray-800 hover:bg-gray-200",
    };

    html! {
        <button
            class=format!("flex items-center justify-center px-3 py-1 rounded font-medium cursor-pointer disabled:bg-opacity-10 {} {}", class_type, props.class)
            onclick=Callback::from(move |_| on_click.emit(()))
        >
            {props.children.clone()}
        </button>
    }
}
