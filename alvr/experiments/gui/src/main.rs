mod dashboard;

use alvr_common::{EventSeverity, Raw, ServerEvent};
use alvr_session::{ClientConnectionDesc, SessionDesc};
use dashboard::Dashboard;
use std::collections::HashSet;

fn load_session() -> SessionDesc {
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

    println!("load_session");

    session
}

fn store_session(session: SessionDesc) {
    println!("store_session");
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
    let dashboard = Dashboard::new();

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
    engine.register_fn("load_session", load_session);
    engine.register_fn("store_session", store_session);
    engine.register_fn("add_client", add_client);
    engine.register_fn("trust_client", trust_client);
    engine.register_fn("remove_client", remove_client);

    let session = load_session();

    dashboard.run(
        session,
        Box::new(move |command| {
            let dynamic_res = engine
                .eval_with_scope::<rhai::Dynamic>(&mut scope, &command)
                .unwrap();

            serde_json::to_value(dynamic_res).unwrap()
        }),
    );
}
