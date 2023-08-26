use alvr_events::Event;
use alvr_packets::ServerRequest;
use eframe::{egui, web_sys};
use ewebsock::{WsEvent, WsMessage, WsReceiver};
use gloo_net::http::Request;

pub struct DataSources {
    context: egui::Context,
    ws_receiver: Option<WsReceiver>,
}

impl DataSources {
    pub fn new(context: egui::Context) -> Self {
        Self {
            context,
            ws_receiver: None,
        }
    }

    pub fn request(&self, request: ServerRequest) {
        let context = self.context.clone();
        wasm_bindgen_futures::spawn_local(async move {
            Request::post("/api/dashboard-request")
                .body(serde_json::to_string(&request).unwrap())
                .send()
                .await
                .ok();

            context.request_repaint();
        })
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        if self.ws_receiver.is_none() {
            let host = web_sys::window().unwrap().location().host().unwrap();
            let Ok((_, receiver)) = ewebsock::connect(format!("ws://{host}/api/events")) else {
                return None;
            };
            self.ws_receiver = Some(receiver);
        }

        if let Some(event) = self.ws_receiver.as_ref().unwrap().try_recv() {
            match event {
                WsEvent::Message(WsMessage::Text(json_string)) => {
                    serde_json::from_str(&json_string).ok()
                }
                WsEvent::Error(_) | WsEvent::Closed => {
                    // recreate the ws connection next poll_event invocation
                    self.ws_receiver = None;

                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn server_connected(&self) -> bool {
        true
    }
}
