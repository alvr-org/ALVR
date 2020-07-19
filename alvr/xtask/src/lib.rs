use fs_extra::{self as fsx, dir as dirx};
use std::{
    env,
    error::Error,
    fs,
    io::{Read, Write},
    path::*,
    process::*,
};

type BResult<T = ()> = Result<T, Box<dyn Error>>;

#[cfg(target_os = "linux")]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_linux";
#[cfg(windows)]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_windows";

#[cfg(target_os = "linux")]
const STEAMVR_OS_DIR_NAME: &str = "linux64";
#[cfg(windows)]
const STEAMVR_OS_DIR_NAME: &str = "win64";

#[cfg(target_os = "linux")]
const DRIVER_FNAME: &str = "driver_alvr_server.so";
#[cfg(windows)]
const DRIVER_FNAME: &str = "driver_alvr_server.dll";

#[cfg(target_os = "linux")]
pub fn exec_fname(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
pub fn exec_fname(name: &str) -> String {
    format!("{}.exe", name)
}

#[cfg(target_os = "linux")]
fn dynlib_fname(name: &str) -> String {
    format!("lib{}.so", name)
}
#[cfg(windows)]
fn dynlib_fname(name: &str) -> String {
    format!("{}.dll", name)
}

fn run_with_args(cmd: &str, args: &[&str]) -> BResult {
    println!(
        "\n{}",
        args.iter().fold(String::from(cmd), |s, arg| s + " " + arg)
    );
    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .spawn()?
        .wait_with_output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn run(cmd: &str) -> BResult {
    let cmd_args = cmd.split_whitespace().collect::<Vec<_>>();
    run_with_args(cmd_args[0], &cmd_args[1..])
}

#[cfg(target_os = "linux")]
fn steamvr_bin_dir() -> BResult<PathBuf> {
    Ok(dirs::home_dir()
        .unwrap()
        .join(".steam/steam/steamapps/common/SteamVR/bin/linux64"))
}
#[cfg(windows)]
pub fn steamvr_bin_dir() -> BResult<PathBuf> {
    use winreg::*;
    let key =
        RegKey::predef(enums::HKEY_CLASSES_ROOT).open_subkey("vrmonitor\\Shell\\Open\\Command")?;
    let command_string: String = key.get_value("")?;

    let path_string = regex::Regex::new(r#""(.+)\\vrmonitor.exe""#)?
        .captures(&command_string)
        .ok_or_else::<Box<dyn Error>, _>(|| "regex failed".into())?
        .get(1)
        .ok_or_else::<Box<dyn Error>, _>(|| "regex failed".into())?
        .as_str();

    Ok(PathBuf::from(path_string))

    // Ok()
    // PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\SteamVR")
}

fn steamvr_dir() -> BResult<PathBuf> {
    Ok(steamvr_bin_dir()?
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into())
}

pub fn target_dir() -> PathBuf {
    Path::new(env!("OUT_DIR")).join("../../../..")
}

pub fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into()
}

pub fn build_dir() -> PathBuf {
    workspace_dir().join("build")
}

pub fn server_build_dir() -> PathBuf {
    build_dir().join(SERVER_BUILD_DIR_NAME)
}

pub fn remove_build_dir() {
    let build_dir = build_dir();
    fs::remove_dir_all(&build_dir).ok();
}

pub fn reset_server_build_folder() -> BResult {
    fs::remove_dir_all(&server_build_dir()).ok();
    fs::create_dir_all(&server_build_dir())?;

    // get all file and folder paths at depth 1, excluded template root (at index 0)
    let dir_content =
        dirx::get_dir_content2("server_release_template", &dirx::DirOptions { depth: 1 })?;
    let items = dir_content.directories[1..]
        .iter()
        .chain(dir_content.files.iter())
        .collect();

    fsx::copy_items(&items, server_build_dir(), &dirx::CopyOptions::new())?;

    Ok(())
}

// https://github.com/mvdnes/zip-rs/blob/master/examples/write_dir.rs
fn zip_dir(dir: &Path) -> BResult {
    let parent_dir = dir.parent().unwrap();
    let zip_file = fs::File::create(parent_dir.join(format!(
        "{}.zip",
        dir.file_name().unwrap().to_string_lossy()
    )))?;
    let mut zip = zip::ZipWriter::new(zip_file);

    let mut buffer = Vec::new();
    let iterator = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok());
    for entry in iterator {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(parent_dir))?;

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            println!("adding file {:?} as {:?} ...", path, name);
            zip.start_file_from_path(name, <_>::default())?;
            let mut f = fs::File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            println!("adding dir {:?} as {:?} ...", path, name);
            zip.add_directory_from_path(name, <_>::default())?;
        }
    }

    Ok(())
}

pub fn build_server(is_release: bool) -> BResult {
    let build_type = if is_release { "release" } else { "debug" };
    let build_flag = if is_release { "--release" } else { "" };

    let target_dir = target_dir();
    let artifacts_dir = target_dir.join(build_type);
    let driver_dst_dir = server_build_dir().join("bin").join(STEAMVR_OS_DIR_NAME);
    let swresample_dir = workspace_dir().join("alvr/server_driver/cpp/libswresample/lib");
    let openvr_api_dir = workspace_dir().join("alvr/server_driver/cpp/openvr/lib");

    reset_server_build_folder()?;
    fs::create_dir_all(&driver_dst_dir)?;

    run("cargo update")?;

    run(&format!(
        "cargo build -p alvr_server_driver -p alvr_web_server -p alvr_server_bootstrap {}",
        build_flag
    ))?;
    fs::copy(
        artifacts_dir.join(dynlib_fname("alvr_server_driver")),
        driver_dst_dir.join(DRIVER_FNAME),
    )?;
    fs::copy(
        swresample_dir.join("avutil-56.dll"),
        driver_dst_dir.join("avutil-56.dll"),
    )?;
    fs::copy(
        swresample_dir.join("swresample-3.dll"),
        driver_dst_dir.join("swresample-3.dll"),
    )?;
    fs::copy(
        openvr_api_dir.join("openvr_api.dll"),
        driver_dst_dir.join("openvr_api.dll"),
    )?;
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_web_server")),
        server_build_dir().join(exec_fname("alvr_web_server")),
    )?;
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_server_bootstrap")),
        server_build_dir().join(exec_fname("ALVR")),
    )?;

    // if cfg!(target_os = "linux") {
    //     use std::io::Write;

    //     let mut shortcut = str_err(
    //         fs::OpenOptions::new()
    //             .append(true)
    //             .open(release_dir.join("alvr.desktop")),
    //     )?;
    //     str_err(writeln!(
    //         shortcut,
    //         "Exec={}",
    //         gui_dst_path.to_string_lossy()
    //     ))?;
    // }

    if is_release {
        zip_dir(&server_build_dir())?;
    }

    Ok(())
}

pub fn build_client(is_release: bool) -> BResult {
    let build_type = if is_release { "release" } else { "debug" };
    let build_task = if is_release {
        "assembleRelease"
    } else {
        "assembleDebug"
    };

    let client_hmd_dir = workspace_dir().join("alvr/client_hmd");
    let command_name = if cfg!(not(windows)) {
        "gradlew"
    } else {
        "gradlew.bat"
    };

    fs::create_dir_all(&build_dir())?;

    env::set_current_dir(&client_hmd_dir)?;
    run(&format!("{} {}", command_name, build_task))?;
    env::set_current_dir(workspace_dir())?;

    fs::copy(
        client_hmd_dir
            .join("app/build/outputs/apk")
            .join(build_type)
            .join(format!("app-{}.apk", build_type)),
        build_dir().join("alvr_client.apk"),
    )?;

    Ok(())
}

pub fn driver_registration(root_server_dir: &Path, register: bool) -> BResult {
    let steamvr_bin_dir = steamvr_bin_dir()?;
    if cfg!(target_os = "linux") {
        env::set_var("LD_LIBRARY_PATH", &steamvr_bin_dir);
    }

    let exec = steamvr_bin_dir.join(exec_fname("vrpathreg"));
    let subcommand = if register {
        "adddriver"
    } else {
        "removedriver"
    };

    let exit_status = Command::new(exec)
        .args(&[subcommand, &root_server_dir.to_string_lossy()])
        .status()?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(format!("Error registering driver: {}", exit_status).into())
    }
}

fn netsh_add_rule_command_string(rule_name: &str, program_path: &Path) -> String {
    format!(
        "netsh advfirewall firewall add rule name=\"{}\" dir=in program=\"{}\" action=allow",
        rule_name,
        program_path.to_string_lossy()
    )
}

fn netsh_delete_rule_command_string(rule_name: &str) -> String {
    format!(
        "netsh advfirewall firewall delete rule name=\"{}\"",
        rule_name,
    )
}

// Errors:
// 1: firewall rule is already set
// other: command failed
pub fn firewall_rules(root_server_dir: &Path, add: bool) -> Result<(), i32> {
    let script_path = env::temp_dir().join("alvr_firewall_rules.bat");

    let firewall_rules_script_content = if add {
        format!(
            "{}\n{}\n{}",
            netsh_add_rule_command_string("ALVR Launcher", root_server_dir),
            netsh_add_rule_command_string(
                "SteamVR ALVR vrserver",
                &steamvr_dir()
                    .map_err(|_| -1)?
                    .join("bin")
                    .join("win64")
                    .join("vrserver.exe")
            ),
            netsh_add_rule_command_string(
                "SteamVR ALVR vrserver",
                &steamvr_dir()
                    .map_err(|_| -1)?
                    .join("bin")
                    .join("win32")
                    .join("vrserver.exe")
            ),
        )
    } else {
        format!(
            "{}\n{}",
            netsh_delete_rule_command_string("ALVR Launcher"),
            netsh_delete_rule_command_string("SteamVR ALVR vrserver")
        )
    };
    fs::write(&script_path, firewall_rules_script_content).map_err(|_| -1)?;

    // run with admin priviles
    let exit_status = runas::Command::new(script_path)
        .show(cfg!(target_os = "linux"))
        .gui(true) // UAC, if available
        .status()
        .map_err(|_| -1)?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap())
    }
}

// Avoid Oculus link popups when debugging the client
pub fn kill_oculus_processes() {
    runas::Command::new("taskkill")
        .args(&["/F", "/IM", "OVR*", "/T"])
        .status()
        .ok();
}

fn get_version(dir_name: &str) -> String {
    let cargo_data = toml::from_slice::<toml::Value>(
        &fs::read(Path::new("..").join(dir_name).join("Cargo.toml")).unwrap(),
    )
    .unwrap();

    cargo_data["package"]["version"].as_str().unwrap().into()
}

pub fn server_version() -> String {
    get_version("server_driver")
}

// pub fn client_version() -> String {
//     get_version("client_hmd")
// }

pub fn get_registered_drivers() -> BResult<Vec<PathBuf>> {
    let output = Command::new(steamvr_bin_dir()?.join(exec_fname("vrpathreg"))).output()?;
    let output = String::from_utf8_lossy(&output.stdout);

    let dirs = regex::Regex::new(r"\t([^\t\r\n]*)")?
        .captures_iter(&output)
        .filter_map(|captures| captures.get(1))
        .map(|cap_match| PathBuf::from(cap_match.as_str()))
        .collect::<Vec<_>>();
    Ok(dirs)
}

pub fn get_alvr_dir() -> BResult<PathBuf> {
    for dir in get_registered_drivers()? {
        if dir.join(exec_fname("ALVR")).exists() && dir.join("web_gui").exists() {
            return Ok(dir);
        }
    }

    Err("ALVR driver is not registered".into())
}

pub fn unregister_all_drivers() -> BResult {
    for dir in get_registered_drivers()? {
        driver_registration(&dir, false)?;
    }

    Ok(())
}
