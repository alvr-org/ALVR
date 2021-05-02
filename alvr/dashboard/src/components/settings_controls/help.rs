use crate::basic_components::IconButton;
use yew::{html, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub text: String,
}

#[function_component(Help)]
pub fn help(Props { text }: &Props) -> Html {
    // todo: add tooltip on hover
    html! {
        <IconButton
            icon_cls="fas fa-question-circle"
        />
    }
}
