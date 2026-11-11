//! Keeps track of server settings that can change mid-stream.
//!
//! The server sends its settings to the client once, when a stream starts. Settings flagged
//! `real-time`, such as hand tracking interaction, take effect on the server as soon as they are
//! changed, so anything the emulator shows about them would go stale until the next reconnect.
//!
//! The server has no request that simply returns its settings. Instead `GET /api/session` makes
//! it broadcast them as an event on its events websocket, which is how the dashboard learns them
//! too. This watcher keeps that websocket open and asks for the session every couple of seconds,
//! and also picks up the broadcasts the dashboard triggers when the user edits a setting.
//!
//! It only looks at the local machine, where the streamer normally runs alongside the emulator.
//! With a remote server it never connects, and the values the stream started with stand.

use alvr_common::{RelaxedAtomic, info, parking_lot::Mutex};
use alvr_session::{SessionConfig, Settings};
use std::{
    io::{ErrorKind, Write},
    net::{SocketAddr, TcpStream},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tungstenite::{client::IntoClientRequest, http::HeaderValue};

/// How often the server is asked to broadcast its session.
const REQUEST_INTERVAL: Duration = Duration::from_secs(2);

/// How long to wait before trying again after the server could not be reached.
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

/// The dashboard's port, used until a stream reports the server's own.
const DEFAULT_PORT: u16 = 8082;

/// Whether the server turns hand gestures into controller input: its "Hand tracking interaction"
/// setting. Gestures are only read while controllers are enabled at all, since the server drives
/// the controller devices from them.
pub fn hand_interaction_enabled(settings: &Settings) -> bool {
    settings
        .headset
        .controllers
        .as_option()
        .is_some_and(|controllers| controllers.hand_tracking_interaction.as_option().is_some())
}

pub struct ServerSettingsWatch {
    port: Arc<Mutex<u16>>,
    hand_interaction: Arc<Mutex<Option<bool>>>,
    running: Arc<RelaxedAtomic>,
}

impl ServerSettingsWatch {
    pub fn start() -> Self {
        let watch = Self {
            port: Arc::new(Mutex::new(DEFAULT_PORT)),
            hand_interaction: Arc::new(Mutex::new(None)),
            running: Arc::new(RelaxedAtomic::new(true)),
        };

        thread::spawn({
            let port = Arc::clone(&watch.port);
            let hand_interaction = Arc::clone(&watch.hand_interaction);
            let running = Arc::clone(&watch.running);

            move || {
                while running.value() {
                    let current_port = *port.lock();
                    watch_once(current_port, &hand_interaction, &running);

                    // Whatever was learned no longer holds once the server is gone.
                    *hand_interaction.lock() = None;
                    thread::sleep(RETRY_INTERVAL);
                }
            }
        });

        watch
    }

    /// Follows the server to the web server port its settings name.
    pub fn set_port(&self, port: u16) {
        *self.port.lock() = port;
    }

    /// The server's hand tracking interaction setting as last broadcast, or `None` when the
    /// server could not be reached.
    pub fn hand_interaction(&self) -> Option<bool> {
        *self.hand_interaction.lock()
    }
}

impl Drop for ServerSettingsWatch {
    fn drop(&mut self) {
        // The thread notices within one read timeout and exits; it is not joined, so closing the
        // window never waits on the network.
        self.running.set(false);
    }
}

/// Connects to the server's events websocket and follows session broadcasts until the
/// connection drops or the watch is stopped.
fn watch_once(port: u16, hand_interaction: &Mutex<Option<bool>>, running: &RelaxedAtomic) {
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    let Ok(socket) = TcpStream::connect_timeout(&address, Duration::from_millis(500)) else {
        return;
    };

    let Ok(mut request) = format!("ws://127.0.0.1:{port}/api/events").into_client_request() else {
        return;
    };
    request
        .headers_mut()
        .insert("X-ALVR", HeaderValue::from_static("true"));

    let Ok((mut websocket, _)) = tungstenite::client(request, socket) else {
        return;
    };

    // A read timeout rather than a non-blocking socket, so the loop sleeps in the read instead of
    // spinning, and still gets round to sending its requests.
    websocket
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(200)))
        .ok();

    info!("Watching the server's settings on port {port}");

    // Due at once, so the first answer arrives straight after connecting.
    let mut next_request = Instant::now();

    while running.value() {
        if Instant::now() >= next_request {
            next_request = Instant::now() + REQUEST_INTERVAL;
            request_session(address);
        }

        match websocket.read() {
            Ok(tungstenite::Message::Text(text)) => {
                if let Some(settings) = session_settings(&text) {
                    let enabled = hand_interaction_enabled(&settings);

                    if hand_interaction.lock().replace(enabled) != Some(enabled) {
                        info!(
                            "Server hand tracking interaction is {}",
                            if enabled { "on" } else { "off" }
                        );
                    }
                }
            }
            Ok(_) => (),
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(_) => return,
        }
    }
}

/// Asks the server to broadcast its session. The answer comes over the websocket, not in the
/// response, which is ignored.
fn request_session(address: SocketAddr) {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(500)) else {
        return;
    };

    stream
        .write_all(
            format!(
                "GET /api/session HTTP/1.1\r\nHost: {address}\r\nX-ALVR: true\r\n\
                 Connection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .ok();
}

/// The settings in a session event, or `None` for any other event. Events are matched by their
/// tag before the session is parsed, since most of the traffic is logs and statistics.
fn session_settings(text: &str) -> Option<Settings> {
    let mut event: serde_json::Value = serde_json::from_str(text).ok()?;
    let event_type = event.get_mut("event_type")?;

    if event_type.get("id")?.as_str()? != "Session" {
        return None;
    }

    let session: SessionConfig = serde_json::from_value(event_type.get_mut("data")?.take()).ok()?;

    Some(session.to_settings())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hand_interaction_from_a_session_event() {
        let mut session = SessionConfig::default();

        for enabled in [false, true] {
            session
                .session_settings
                .headset
                .controllers
                .content
                .hand_tracking_interaction
                .enabled = enabled;

            let event = serde_json::json!({
                "timestamp": "",
                "event_type": { "id": "Session", "data": session },
            });

            let settings = session_settings(&event.to_string()).expect("a session event");
            assert_eq!(hand_interaction_enabled(&settings), enabled);
        }

        let log = serde_json::json!({
            "timestamp": "",
            "event_type": { "id": "Log", "data": { "severity": "Info", "content": "" } },
        });
        assert!(session_settings(&log.to_string()).is_none());
    }
}
