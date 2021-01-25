use alvr_common::logging::show_err;
use alvr_server::*;
use std::{sync::Once, thread, time::Duration};

fn main() {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(|| {
        show_err(init());
    });

    driver_ready_idle();

    thread::sleep(Duration::from_secs(100000000000000000));
}
