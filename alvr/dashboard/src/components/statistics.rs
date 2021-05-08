use yew::html;
use yew_functional::{function_component, use_state};

#[function_component(Statistics)]
pub fn statistics() -> Html {
    let maybe_statistics_handle = use_state(|| None);

    use_state({
        let maybe_statistics_handle = maybe_statistics_handle.clone();
        || crate::recv_event_cb!(Statistics, |stats| maybe_statistics_handle.set(Some(stats)))
    });

    if let Some(statistics) = &*maybe_statistics_handle {
        html! {
            {"statistics"}
        }
    } else {
        html!({ "No statistics available" })
    }
}
