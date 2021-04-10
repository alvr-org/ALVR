use crate::basic_components::{Button, ButtonGroup, ButtonType, Select, Slider, Switch};
use std::rc::Rc;
use yew::{html, Callback};
use yew_functional::{function_component, use_state};

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let (label, set_label) = use_state(|| "Hello".to_owned());

    let on_click = {
        let label = Rc::clone(&label);
        Callback::from(move |_| set_label(format!("{} world", label)))
    };

    let switch_on_click = Callback::from(move |_| ());

    let slider_on_change = Callback::from(move |_| ());

    let on_select = Callback::from(move |_| ());

    html! {
        <>
            <Button on_click=on_click button_type=ButtonType::None>
                {label}
            </Button>
            <Switch on_click=switch_on_click checked=true/>
            <Slider value="0" default="30" min="-1" max="40" step="0.5" on_change=slider_on_change/>
            <ButtonGroup
                options=vec!["hello1".into(), "hello2".into()]
                selected="hello1"
                on_select=on_select.clone()
            />
            <Select
                options=vec!["hello1".into(), "hello2".into()]
                selected="hello1"
                on_select=on_select
            />
        </>
    }
}
