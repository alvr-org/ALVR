use settings_schema::HigherOrderType;
use yew::{html, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub data_type: HigherOrderType,
    pub modifiers: Vec<String>,
}

#[function_component(HigherOrder)]
pub fn higher_order(props: &Props) -> Html {
    html!("higher order")
}
