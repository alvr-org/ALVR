mod dashboard;

use alvr_session::{ClientConnectionDesc, SessionDesc};
use dashboard::Dashboard;
use std::collections::HashSet;

fn main() {
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

    let dashboard = Dashboard::new(session);

    dashboard.run(|event| match event {
        _ => (),
    });
}
