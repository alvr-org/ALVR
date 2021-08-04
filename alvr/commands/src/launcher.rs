use alvr_common::prelude::*;
use std::{path::Path, process::Command};

fn invoke_launcher(launcher_path: &Path, flag: &str) -> StrResult {
    trace_err!(Command::new(launcher_path).arg(flag).status())?;

    Ok(())
}

pub fn restart_steamvr(launcher_path: &Path) -> StrResult {
    invoke_launcher(launcher_path, "--restart-steamvr")
}

pub fn invoke_application_update(launcher_path: &Path) -> StrResult {
    invoke_launcher(launcher_path, "--update")
}
