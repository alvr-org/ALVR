mod commands;

use std::{sync::mpsc, thread, time::Duration};

use crate::WorkerMsg;

pub fn launcher_thread(rx1: mpsc::Receiver<WorkerMsg>) {
    let mut tried_steamvr_launch = false;

    loop {
        for msg in rx1.try_iter() {
            match msg {
                WorkerMsg::LostConnection(_) => {
                    if !tried_steamvr_launch {
                        if alvr_common::show_err(commands::maybe_register_alvr_driver()).is_some() {
                            if commands::is_steamvr_running() {
                                commands::kill_steamvr();
                                thread::sleep(Duration::from_secs(2))
                            }
                            commands::maybe_launch_steamvr();
                        }
                        tried_steamvr_launch = true;
                    }
                }
                _ => {}
            }
        }
    }
}
