use crate::events_dispatch;
use std::{collections::VecDeque, rc::Rc};
use yew::{html, Callback};
use yew_functional::{function_component, use_state};

const MAX_LOG_COUNT: usize = 50;

#[function_component(Logs)]
pub fn logs() -> Html {
    let (log_list, set_log_list) = use_state(VecDeque::new);

    use_state({
        let log_list = Rc::clone(&log_list);
        move || {
            events_dispatch::recv_any_event_cb(Callback::from(move |event| {
                let mut log_list = (*log_list).clone();

                log_list.push_back(event);
                if log_list.len() > MAX_LOG_COUNT {
                    log_list.pop_front();
                }

                set_log_list(log_list)
            }))
        }
    });

    html! {
        {format!("There are {} log entries", log_list.len())}
    }
}
