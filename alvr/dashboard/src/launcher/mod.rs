mod commands;

use std::{sync::mpsc, thread, time::Duration};

use crate::WorkerMsg;

pub fn launch() {
    if alvr_common::show_err(commands::maybe_register_alvr_driver()).is_some() {
        if commands::is_steamvr_running() {
            commands::kill_steamvr();
            thread::sleep(Duration::from_secs(2))
        }
        commands::maybe_launch_steamvr();
    }
}
