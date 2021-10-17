mod dashboard;

use alvr_session::SessionDesc;
use dashboard::{Dashboard, DashboardEvent};

fn main() {
    let dashboard = Dashboard::new(SessionDesc::default());

    dashboard.run(|event| {
        match event {
            _ => ()
        }
    });
}