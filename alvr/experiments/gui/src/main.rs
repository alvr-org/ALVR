mod dashboard;

use alvr_common::{EventSeverity, Raw, ServerEvent};
use alvr_session::{ClientConnectionDesc, SessionDesc};
use dashboard::Dashboard;
use std::collections::HashSet;

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

    let engine = rhai::Engine::new();

    let session = rhai::serde::to_dynamic(SessionDesc::default()).unwrap();

    let mut scope = rhai::Scope::new();
    scope.push_dynamic("session", session);

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

    dashboard.run(
        session,
        Box::new(move |command| {
            engine
                .eval_with_scope::<rhai::Dynamic>(&mut scope, &command)
                .map(|d| d.to_string())
                .unwrap_or_else(|e| e.to_string())
        }),
    );
}
