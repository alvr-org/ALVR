use fs_extra::{self as fsx, dir as dirx};
use std::{env, error::Error, fs, path::*, process::*};

type BResult<T = ()> = Result<T, Box<dyn Error>>;

pub const DEFAULT_PORTS: [u16; 2] = [9943, 9944];

const NIGHTLY_TOOLCHAIN_VERSION: &str = "nightly-2020-04-30";

#[cfg(target_os = "linux")]
const DRIVER_REL_DIR_STR: &str = "bin/linux64";
#[cfg(windows)]
const DRIVER_REL_DIR_STR: &str = "bin/win64";

#[cfg(target_os = "linux")]
const DRIVER_FNAME: &str = "driver_alvr_server.so";
#[cfg(windows)]
const DRIVER_FNAME: &str = "driver_alvr_server.dll";

#[cfg(target_os = "linux")]
fn exec_fname(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
fn exec_fname(name: &str) -> String {
    format!("{}.exe", name)
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
            "\nCommand failed\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn run(cmd: &str) -> BResult {
    let cmd_args = cmd.split_whitespace().collect::<Vec<_>>();
    run_with_args(cmd_args[0], &cmd_args[1..])
}

fn path_to_string(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

fn msbuild_path() -> PathBuf {
    PathBuf::from(
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\MSBuild\Current\Bin\MSBuild.exe",
    )
}

#[cfg(target_os = "linux")]
fn steamvr_bin_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join(".steam/steam/steamapps/common/SteamVR/bin/linux64")
}
#[cfg(windows)]
fn steamvr_bin_dir() -> PathBuf {
    PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/SteamVR/bin/win64")
}

pub fn target_dir() -> PathBuf {
    Path::new(env!("OUT_DIR")).join("../../../..")
}

pub fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn build_dir() -> PathBuf {
    workspace_dir().join("build")
}

pub fn server_build_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    const OS_STR: &str = "linux";
    #[cfg(windows)]
    const OS_STR: &str = "windows";

    build_dir().join(format!("alvr_server_{}", OS_STR))
}

pub fn remove_build_dir() {
    let build_dir = build_dir();
    fs::remove_dir_all(&build_dir).ok();
}

pub fn reset_server_build_folder() -> BResult {
    fs::create_dir_all(build_dir())?;
    let server_build_dir = server_build_dir();
    fs::remove_dir_all(&server_build_dir).ok();

    fs::create_dir_all(&server_build_dir)?;

    // get all file and folder paths at depth 1, excluded template root (at index 0)
    let dir_content =
        dirx::get_dir_content2("server_release_template", &dirx::DirOptions { depth: 1 })?;
    let items = dir_content.directories[1..]
        .iter()
        .chain(dir_content.files.iter())
        .collect();

    fsx::copy_items(&items, server_build_dir, &dirx::CopyOptions::new())?;

    Ok(())
}

pub fn build_server(is_release: bool) -> BResult {
    let build_type = if is_release { "release" } else { "debug" };
    let build_flag = if is_release { "--release" } else { "" };

    let server_driver_dir = workspace_dir().join("alvr/server_driver");
    let target_dir = target_dir();
    let artifacts_dir = target_dir.join(build_type);
    let build_dir = build_dir();
    let lib_build_dir = build_dir.join("lib");
    let server_build_dir = server_build_dir();
    let driver_dst_dir = server_build_dir.join(DRIVER_REL_DIR_STR);

    reset_server_build_folder()?;
    fs::create_dir_all(&lib_build_dir)?;
    fs::create_dir_all(&driver_dst_dir)?;

    run(&format!(
        "cargo build -p alvr_server_driver_ext {}",
        build_flag
    ))?;
    println!("Please wait for cbindgen...");
    run(&format!(
        "rustup run {} cbindgen --config alvr/server_driver_ext/cbindgen.toml \
            --crate alvr_server_driver_ext --output build/include/alvr_server_driver_ext.h",
        NIGHTLY_TOOLCHAIN_VERSION
    ))?;

    if cfg!(windows) {
        fs::copy(
            artifacts_dir.join("alvr_server_driver_ext.lib"),
            lib_build_dir.join("alvr_server_driver_ext.lib"),
        )?;
        run_with_args(
            &msbuild_path().to_string_lossy(),
            &[
                "alvr/server_driver/ALVR.sln",
                &format!("-p:Configuration={}", build_type),
                "-p:Platform=x64",
            ],
        )?;
        fs::copy(
            server_driver_dir
                .join("x64")
                .join(build_type)
                .join(DRIVER_FNAME),
            driver_dst_dir.join(DRIVER_FNAME),
        )?;
    }

    run(&format!("cargo build -p alvr_web_server {}", build_flag))?;
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_web_server")),
        server_build_dir.join(exec_fname("alvr_web_server")),
    )?;
    run(&format!(
        "cargo build -p alvr_server_bootstrap {}",
        build_flag
    ))?;
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_server_bootstrap")),
        server_build_dir.join(exec_fname("ALVR")),
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

    Ok(())
}

pub fn driver_registration(root_server_dir: &Path, register: bool) -> BResult {
    let steamvr_bin_dir = steamvr_bin_dir();
    if cfg!(target_os = "linux") {
        env::set_var("LD_LIBRARY_PATH", &steamvr_bin_dir);
    }

    let exec = steamvr_bin_dir.join(exec_fname("vrpathreg"));
    let cmd = if register {
        "adddriver"
    } else {
        "removedriver"
    };
    run(&format!(
        "{} {} {}",
        path_to_string(&exec),
        cmd,
        path_to_string(&root_server_dir)
    ))
}

pub fn firewall_rules(root_server_dir: &Path, add: bool) -> BResult {
    let script_fname = if add {
        "add_firewall_rules.bat"
    } else {
        "remove_firewall_rule.bat"
    };

    let script_path = path_to_string(&root_server_dir.join(script_fname));

    // run with admin priviles
    let exit_status = runas::Command::new(script_path)
        .arg("/s")
        .show(cfg!(target_os = "linux"))
        .gui(true) // UAC, if available
        .status()?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(format!("\nCommand failed\n{}", exit_status).into())
    }
}

fn get_version(dir_name: &str) -> String {
    let cargo_data = toml::from_slice::<toml::Value>(
        &fs::read(Path::new("..").join(dir_name).join("Cargo.toml")).unwrap(),
    )
    .unwrap();

    cargo_data["package"]["version"].as_str().unwrap().into()
}

pub fn server_version() -> String {
    get_version("server_driver_ext")
}

// pub fn client_version() -> String {
//     get_version("client_hmd")
// }

pub fn install_deps() -> BResult {
    run(&format!(
        "rustup toolchain install {}",
        NIGHTLY_TOOLCHAIN_VERSION
    ))?;
    run(&format!(
        "cargo +{} install cbindgen --force",
        NIGHTLY_TOOLCHAIN_VERSION
    ))
}

pub fn get_alvr_dir_using_vrpathreg() -> BResult<PathBuf> {
    let output = Command::new(steamvr_bin_dir().join(exec_fname("vrpathreg"))).output()?;
    let output = String::from_utf8_lossy(&output.stdout);

    let maybe_captures = regex::Regex::new(r"^\t(.*)$")?
        .captures_iter(&output)
        .last();
    if let Some(captures) = maybe_captures {
        if let Some(cap_match) = captures.get(1) {
            return Ok(PathBuf::from(cap_match.as_str()));
        }
    }
    Err("No directory found".into())
}
