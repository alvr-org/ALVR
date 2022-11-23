use alvr_common::prelude::*;
use alvr_filesystem as afs;
use serde_json as json;
use std::{
    env, fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

#[cfg(windows)]
use sysinfo::PidExt;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn spawn_no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW).spawn().ok();
}

#[cfg(not(windows))]
fn spawn_no_window(command: &mut Command) {
    command.spawn().ok();
}

pub fn is_steamvr_running() -> bool {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    system
        .processes_by_name(&afs::exec_fname("vrserver"))
        .count()
        != 0
}

pub fn maybe_launch_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    if system
        .processes_by_name(&afs::exec_fname("vrserver"))
        .count()
        == 0
    {
        #[cfg(windows)]
        spawn_no_window(Command::new("cmd").args(["/C", "start", "steam://rungameid/250820"]));
        #[cfg(not(windows))]
        spawn_no_window(Command::new("steam").args(["steam://rungameid/250820"]));
    }
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::os::windows::process::CommandExt;
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

// this will not kill the child process "ALVR launcher"
pub fn kill_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.processes_by_name(&afs::exec_fname("vrmonitor")) {
        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());
    }

    thread::sleep(Duration::from_secs(1));

    for process in system.processes_by_name(&afs::exec_fname("vrserver")) {
        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());
    }
}

pub fn check_steamvr_installation() -> bool {
    alvr_commands::openvr_source_file_path().is_ok()
}

pub fn unblock_alvr_addon() -> StrResult {
    let config_path = alvr_commands::steam_config_dir()?.join("steamvr.vrsettings");

    let mut fields_ref: json::Map<String, json::Value> =
        json::from_str(&fs::read_to_string(&config_path).map_err(err!())?).map_err(err!())?;

    fields_ref.remove("driver_alvr_server");

    fs::write(
        config_path,
        json::to_string_pretty(&fields_ref).map_err(err!())?,
    )
    .map_err(err!())?;

    Ok(())
}

pub fn maybe_register_alvr_driver() -> StrResult {
    let alvr_driver_dir = afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
        .openvr_driver_root_dir;

    let driver_registered = alvr_commands::get_driver_dir_from_registered()
        .ok()
        .filter(|dir| *dir == alvr_driver_dir)
        .is_some();

    if !driver_registered {
        let paths_backup = match alvr_commands::get_registered_drivers() {
            Ok(paths) => paths,
            Err(e) => {
                return fmt_e!(
                "{}\n{}\n\n({})",
                "Failed to load registered drivers.",
                "Please reset the drivers installation with the apposite button on the launcher.",
                e
            )
            }
        };

        alvr_commands::maybe_save_driver_paths_backup(&paths_backup)?;

        alvr_commands::driver_registration(&paths_backup, false)?;

        alvr_commands::driver_registration(&[alvr_driver_dir], true)?;
    }

    #[cfg(target_os = "linux")]
    maybe_wrap_vrcompositor_launcher()?;
    #[cfg(target_os = "linux")]
    maybe_fix_vrenv_dylib_path()?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn maybe_wrap_vrcompositor_launcher() -> StrResult {
    let steamvr_bin_dir = alvr_commands::steamvr_root_dir()?
        .join("bin")
        .join("linux64");
    let real_launcher_path = steamvr_bin_dir.join("vrcompositor.real");
    let launcher_path = steamvr_bin_dir.join("vrcompositor");

    // In case of SteamVR update, vrcompositor will be restored
    match fs::read_link(&launcher_path) {
        Err(_) => match fs::metadata(&launcher_path) {
            Err(_) => (), //file does not exist, do nothing
            Ok(_) => {
                fs::rename(&launcher_path, &real_launcher_path).map_err(err!())?;
            }
        },
        Ok(_) => fs::remove_file(&launcher_path).map_err(err!())?, // recreate the link
    };

    std::os::unix::fs::symlink(
        afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
            .vrcompositor_wrapper(),
        &launcher_path,
    )
    .map_err(err!())?;

    Ok(())
}

// On nixpkgs (and hence NixOS) Steam is packaged in a
// container with various Valve-provided libraries.
//
// Their ffmpeg lacks vulkan support and we have to force
// our way out of it by modifying the `vrenv.sh` script.
//
// (And luckily, nixpkgs' container is very loose, so
// we can get out of it by simply using /nix/store)
#[cfg(target_os = "linux")]
pub fn maybe_fix_vrenv_dylib_path() -> StrResult {
    use std::{
        fs::File, io::Write, os::unix::prelude::PermissionsExt, path::PathBuf, str::FromStr,
    };

    // Is this system weird?
    {
        let steam_path = which::which("steam").map_err(err!())?;
        let steam_path_s = steam_path
            .canonicalize()
            .map_err(err!())?
            .to_str()
            .ok_or("steam binary path wasn't valid. non-unicode?")?
            .to_string();

        if !["/nix/store", "/gnu/store"]
            .iter()
            .any(|p| steam_path_s.starts_with(p))
        {
            return Ok(());
        }
    }

    // defined in xtask
    let ffmpeg_path = PathBuf::from_str(env!("ALVR_FFMPEG_PATH")).unwrap();

    // Modifying our file handling the various edge cases
    // from steam updates and previous runs.
    {
        // ACHTUNG! changing this may brick existing installations!
        let watermark = "# automatically patched by ALVR! be careful (:\n";

        let vrenv_path = alvr_commands::steamvr_root_dir()?
            .join("bin")
            .join("vrenv.sh");
        assert!(vrenv_path.is_file());
        let mut work = fs::read_to_string(&vrenv_path).map_err(err!())?;

        let vrenv_backup_path = vrenv_path.with_file_name("vrenv.sh.bak");
        let has_watermark = work.contains(watermark);

        let copy_replacing = |src: &PathBuf, dest: &PathBuf| -> StrResult {
            if dest.exists() {
                fs::remove_file(dest).map_err(err!())?;
            }
            fs::copy(src, dest).map_err(err!())?;
            Ok(())
        };
        if has_watermark {
            // Restore from the backup before we modify it again..
            copy_replacing(&vrenv_backup_path, &vrenv_path)?;
            work = fs::read_to_string(&vrenv_path).map_err(err!())?;
        } else {
            // It's unmodified, we can create a backup..
            copy_replacing(&vrenv_path, &vrenv_backup_path)?;
        }

        // Please forgive me..
        work = work.replace(
            r#"export LD_LIBRARY_PATH=""#,
            &format!(r#"export LD_LIBRARY_PATH="{}/lib:"#, ffmpeg_path.display()),
        );
        work = work.replace("#!/bin/bash\n", &format!("#!/bin/bash\n{watermark}"));
        fs::remove_file(&vrenv_path).map_err(err!())?;
        let mut f = File::create(&vrenv_path).map_err(err!())?;
        f.write_all(work.as_bytes()).map_err(err!())?;
        // Last minute fixup fixup: make it executable again..
        let mut m = f.metadata().map_err(err!())?.permissions();
        m.set_mode(&m.mode() | 0o110);
        f.set_permissions(m).map_err(err!())?;
    }

    Ok(())
}

pub fn fix_steamvr() {
    // If ALVR driver does not start use a more destructive approach: delete openvrpaths.vrpath then recreate it
    if let Ok(path) = alvr_commands::openvr_source_file_path() {
        fs::remove_file(path).ok();

        maybe_launch_steamvr();
        thread::sleep(Duration::from_secs(5));
        kill_steamvr();
        thread::sleep(Duration::from_secs(5));
    }

    unblock_alvr_addon().ok();
}

fn try_close_steamvr_gracefully() {
    let start_time = Instant::now();
    while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
        thread::sleep(Duration::from_millis(500));
    }

    // Note: if SteamVR already shutdown cleanly, this does nothing
    kill_steamvr();

    thread::sleep(Duration::from_secs(2));
}

pub fn restart_steamvr() {
    try_close_steamvr_gracefully();

    if alvr_common::show_err(maybe_register_alvr_driver()).is_some() {
        maybe_launch_steamvr();
    }
}

pub fn invoke_installer() {
    try_close_steamvr_gracefully();

    spawn_no_window(Command::new(afs::installer_path()).arg("-q"));

    // delete crash_log.txt (take advantage of the occasion to do some routine cleaning)
    fs::remove_file(
        afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap()).crash_log(),
    )
    .ok();
}
