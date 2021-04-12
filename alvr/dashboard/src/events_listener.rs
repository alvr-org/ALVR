use alvr_common::{logging::Event, prelude::*};
use futures::{
    future::{self, Either},
    stream::StreamExt,
};
use pharos::{Filter, Observable};
use std::future::Future;
use web_sys::window;
use ws_stream_wasm::{WsEvent, WsMessage, WsMeta};

pub async fn events_listener<F: Future<Output = StrResult>>(
    callback: impl Fn(Event) -> F,
) -> StrResult {
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
                callback(event).await?;
            }

            Ok(())
        };

        let mut events = trace_err!(ws.observe(Filter::Pointer(WsEvent::is_closed).into()).await)?;
        let ws_closed_future = async {
            if let Some(WsEvent::Closed(event)) = events.next().await {
                info!(
                    "Event Event websocket closed. Reason: [{}] Reopening...",
                    event.reason
                )
            }
        };

        futures::pin_mut!(messages_loop, ws_closed_future);
        let result = future::select(messages_loop, ws_closed_future).await;

        if let Either::Left((res, _)) = result {
            break res;
        }
    }
}
