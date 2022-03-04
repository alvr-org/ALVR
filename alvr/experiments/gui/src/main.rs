use alvr_common::lazy_static;
use alvr_gui::Dashboard;
use alvr_session::{ClientConnectionDesc, EventSeverity, Raw, ServerEvent, SessionDesc};
use rhai::Dynamic;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

lazy_static! {
    static ref SESSION: Arc<Mutex<SessionDesc>> = {
        let mut session = SessionDesc::default();
        session.client_connections.insert(
            "1234.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: false,
            },
        );
        session.client_connections.insert(
            "4321.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );
        session.client_connections.insert(
            "51423.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );

        Arc::new(Mutex::new(session))
    };
    static ref SESSION_MODIFIED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

fn load_session() -> SessionDesc {
    SESSION.lock().unwrap().clone()
}

fn load_session_dyn() -> Dynamic {
    rhai::serde::to_dynamic(load_session()).unwrap()
}

fn store_session(session: Dynamic) -> String {
    match rhai::serde::from_dynamic(&session) {
        Ok(res) => {
            *SESSION.lock().unwrap() = res;
            SESSION_MODIFIED.store(true, Ordering::Relaxed);

            println!("store_session");

            "".into()
        }
        Err(e) => e.to_string(),
    }
}

fn add_client(hostname: &str, ip: &str) {
    println!("add_client");
}

fn trust_client(hostname: &str) {
    println!("trust_client");
}

fn remove_client(hostname: &str) {
    println!("remove_client");
}

fn main() {
    let dashboard = Arc::new(Dashboard::new());

    dashboard.report_event(ServerEvent::Raw(Raw {
        timestamp: "time1".into(),
        severity: EventSeverity::Info,
        content: "test1".into(),
    }));
    dashboard.report_event(ServerEvent::Raw(Raw {
        timestamp: "time2".into(),
        severity: EventSeverity::Warning,
        content: "test2".into(),
    }));
    dashboard.report_event(ServerEvent::Raw(Raw {
        timestamp: "time3".into(),
        severity: EventSeverity::Error,
        content: "test3".into(),
    }));
    dashboard.report_event(ServerEvent::Raw(Raw {
        timestamp: "time4".into(),
        severity: EventSeverity::Debug,
        content: "test4".into(),
    }));

    let mut engine = rhai::Engine::new();

    let mut scope = rhai::Scope::new();
    engine.register_fn("load_session", load_session_dyn);
    engine.register_fn("store_session", store_session);
    engine.register_fn("add_client", add_client);
    engine.register_fn("trust_client", trust_client);
    engine.register_fn("remove_client", remove_client);

    dashboard.run(load_session(), {
        let dashboard = Arc::clone(&dashboard);
        Box::new(move |command| {
            let res = engine
                .eval_with_scope::<rhai::Dynamic>(&mut scope, &command)
                .map(|d| d.to_string())
                .map_err(|e| e.to_string());

            if SESSION_MODIFIED.load(Ordering::Relaxed) {
                dashboard.report_event(ServerEvent::Session(Box::new(
                    SESSION.lock().unwrap().clone(),
                )))
            }

            res
        })
    });
}
