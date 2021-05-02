use yew::{html, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub text: String,
}

#[function_component(Notice)]
pub fn notice(Props { text }: &Props) -> Html {
    // todo: put text inside a card. Use a yellow border
    html! {
        <div> {text} </div>
    }
}
