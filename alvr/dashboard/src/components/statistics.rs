use yew::html;
use yew_functional::{function_component, use_state};

#[function_component(Statistics)]
pub fn statistics() -> Html {
    let (maybe_statistics, set_statistics) = use_state(|| None);

    use_state(|| crate::recv_event_cb!(Statistics, |stats| set_statistics(Some(stats))));

    if let Some(statistics) = &*maybe_statistics {
        html! {
            {"statistics"}
        }
    } else {
        html!({ "No statistics available" })
    }
}
