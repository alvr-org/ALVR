use crate::events_dispatch;
use std::collections::VecDeque;
use yew::{html, Callback};
use yew_functional::{function_component, use_state};

const MAX_LOG_COUNT: usize = 50;

#[function_component(Logs)]
pub fn logs() -> Html {
    let log_list = use_state(VecDeque::new);

    use_state({
        let log_list = log_list.clone();
        move || {
            events_dispatch::recv_any_event_cb(Callback::from(move |event| {
                let mut log_list_copy = (*log_list).clone();

                log_list_copy.push_back(event);
                if log_list_copy.len() > MAX_LOG_COUNT {
                    log_list_copy.pop_front();
                }

                log_list.set(log_list_copy)
            }))
        }
    });

    html! {
        {format!("There are {} log entries", log_list.len())}
    }
}
