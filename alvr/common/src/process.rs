use alvr_xtask::*;
use std::{path::Path, process::*};
use sysinfo::*;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// Launch web server. If another instance exists, the one just spawned will close itself.
pub fn maybe_launch_web_server(root_server_dir: &Path) {
    let mut command = Command::new(root_server_dir.join("alvr_web_server"));

    // somehow the console is always empty, so it's useless
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command.spawn().ok();
}

#[cfg(windows)]
fn kill_process(pid: usize) {
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

// Kill web server and its child processes if only one of bootstrap or driver is alive.
pub fn maybe_kill_web_server() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    let bootstrap_or_driver_count = system.get_process_by_name(&exec_fname("ALVR")).len()
        + system.get_process_by_name(&exec_fname("vrserver")).len();

    if bootstrap_or_driver_count <= 1 {
        for process in system.get_processes().values() {
            if let Some(parent_pid) = process.parent() {
                if let Some(parent_proc) = system.get_process(parent_pid) {
                    if parent_proc.name() == exec_fname("alvr_web_server") {
                        // Using built-in method causes cmd to pop up repeatedly on Windows
                        #[cfg(not(windows))]
                        process.kill(Signal::Term);
                        #[cfg(windows)]
                        kill_process(process.pid());
                    }
                }
            }
        }
        for process in system.get_process_by_name(&exec_fname("alvr_web_server")) {
            #[cfg(not(windows))]
            process.kill(Signal::Term);
            #[cfg(windows)]
            kill_process(process.pid());
        }
    }
}

pub fn launch_steamvr() {
    Command::new(steamvr_bin_dir().join(exec_fname("vrmonitor")))
        .spawn()
        .ok();
}
