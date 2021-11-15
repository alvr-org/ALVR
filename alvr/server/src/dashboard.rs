use crate::{graphics_info, ClientListAction, NEW_DASHBOARD, CHROME_DASHBOARD, SESSION_MANAGER};
use alvr_common::prelude::*;
use alvr_gui::Dashboard;
use alvr_session::SessionDesc;
use parking_lot::Mutex;
use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::broadcast::{error::RecvError, Sender};

lazy_static::lazy_static! {
    static ref TEMP_SESSION: Arc<Mutex<SessionDesc>> = Arc::new(Mutex::new(SessionDesc::default()));
    static ref TEMP_SESSION_MODIFIED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub async fn event_listener(events_sender: Sender<String>) {
    let mut receiver = events_sender.subscribe();

    loop {
        match receiver.recv().await {
            Ok(event) => {
                if let Some(dashboard) = &*NEW_DASHBOARD.lock() {
                    dashboard.report_event(serde_json::from_str(&event).unwrap());
                }
            }
            Err(RecvError::Lagged(_)) => {
                // warn!("Some log lines have been lost because the buffer is full");
            }
            Err(RecvError::Closed) => break,
        }
    }
}

fn load_session() -> rhai::Dynamic {
    rhai::serde::to_dynamic(TEMP_SESSION.lock().clone()).unwrap()
}

fn store_session(session: rhai::Dynamic) -> String {
    match rhai::serde::from_dynamic(&session) {
        Ok(res) => {
            *TEMP_SESSION.lock() = res;
            TEMP_SESSION_MODIFIED.store(true, Ordering::Relaxed);

            "".into()
        }
        Err(e) => e.to_string(),
    }
}

fn add_client(hostname: String, display_name: String) {
    crate::update_client_list(hostname, ClientListAction::AddIfMissing { display_name });
}

fn trust_client(hostname: String) {
    crate::update_client_list(hostname, ClientListAction::TrustAndMaybeAddIp(None));
}

fn remove_client(hostname: String) {
    crate::update_client_list(hostname, ClientListAction::RemoveIpOrEntry(None));
}

#[cfg(feature = "new-dashboard")]
pub fn ui_thread() -> StrResult {
    let mut engine = rhai::Engine::new();

    let mut scope = rhai::Scope::new();
    engine.register_fn("load_session", load_session);
    engine.register_fn("store_session", store_session);
    engine.register_fn("add_client", add_client);
    engine.register_fn("trust_client", trust_client);
    engine.register_fn("remove_client", remove_client);

    let dashboard = Arc::new(Dashboard::new());

    *MAYBE_NEW_DASHBOARD.lock() = Some(Arc::clone(&dashboard));

    dashboard.run(
        SESSION_MANAGER.lock().get().clone(),
        Box::new(move |command| {
            // Each time the handler is invoked, the command might request access to the session.
            // Keep the session manager locked during the evaluation of the command to avoid race
            // conditions
            let mut session_manager = SESSION_MANAGER.lock();
            *TEMP_SESSION.lock() = session_manager.get().clone();

            let res = engine
                .eval_with_scope::<rhai::Dynamic>(&mut scope, &command)
                .map(|d| d.to_string())
                .map_err(|e| e.to_string());

            // Save session only of modified. This will also generates the Session() event that
            // refreshes the dashboard
            if TEMP_SESSION_MODIFIED.load(Ordering::Relaxed) {
                *session_manager.get_mut() = TEMP_SESSION.lock().clone();
            }

            res
        }),
    );

    crate::shutdown_runtime();
    unsafe { crate::ShutdownSteamvr() };

    Ok(())
}
