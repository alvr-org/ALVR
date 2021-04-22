use alvr_common::{logging::Event, prelude::*};
use futures::{
    future::{self, Either},
    stream::StreamExt,
};
use pharos::{Filter, Observable};
use std::{cell::RefCell, collections::HashMap};
use web_sys::window;
use ws_stream_wasm::{WsEvent, WsMessage, WsMeta};
use yew::Callback;

thread_local! {
    static LISTENERS: RefCell<HashMap<String, Callback<Event>>> = RefCell::new(HashMap::new())
}

pub async fn events_dispatch_loop() -> StrResult {
    loop {
        let (mut ws, mut wsio) = trace_err!(
            WsMeta::connect(
                &format!(
                    "ws://{}/api/events",
                    trace_err_dbg!(trace_none!(window())?.location().host())?
                ),
                None,
            )
            .await
        )?;

        let messages_loop = async {
            while let Some(WsMessage::Text(text)) = wsio.next().await {
                let event = trace_err!(serde_json::from_str::<Event>(&text))?;

                LISTENERS.with(|listeners| {
                    for listener in listeners.borrow_mut().values() {
                        listener.emit(event.clone());
                    }
                });
            }

            Ok(())
        };

        let mut events = trace_err!(ws.observe(Filter::Pointer(WsEvent::is_closed).into()).await)?;
        let ws_closed_future = async {
            if let Some(WsEvent::Closed(event)) = events.next().await {
                info!(
                    "Event websocket closed. Reason: [{}] Reopening...",
                    event.reason
                )
            }
        };

        futures::pin_mut!(messages_loop, ws_closed_future);
        let result = future::select(messages_loop, ws_closed_future).await;

        if let Either::Left((res, _)) = result {
            // unexpected error in message_loop -> stop
            break res;
        } else {
            // ws_closed_future terminated -> retry
            continue;
        }
    }
}

// Dropping this handle will unregister the listener
pub struct ListenerHandle {
    id: String,
}

impl Drop for ListenerHandle {
    fn drop(&mut self) {
        LISTENERS.with(|listeners| {
            listeners.borrow_mut().remove(&self.id);
        });
    }
}

pub fn recv_any_event_cb(callback: Callback<Event>) -> ListenerHandle {
    LISTENERS.with(|listeners| {
        let id = crate::get_id();

        listeners.borrow_mut().insert(id.clone(), callback);

        ListenerHandle { id }
    })
}

// Note: the body is repeated for zero or some arguments, because "||" is treated as a single token
#[macro_export]
macro_rules! recv_event_cb {
    ($event:ident, || $callback_body:expr) => {
        crate::events_dispatch::recv_any_event_cb(yew::Callback::from(move |event| {
            if let alvr_common::logging::Event::$event = event {
                $callback_body
            }
        }))
    };
    ($event:ident, |$($args:ident),+| $callback_body:expr) => {
        crate::events_dispatch::recv_any_event_cb(yew::Callback::from(move |event| {
            if let alvr_common::logging::Event::$event($($args),+) = event {
                $callback_body
            }
        }))
    };
}
