use alvr_common::data::SessionDesc;
use std::rc::Rc;
use yew::{html, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub session: Rc<SessionDesc>,
}

#[function_component(Connections)]
pub fn connections(props: &Props) -> Html {
    html! {
        {"connections"}
    }
}
