mod dependencies;
mod version;

use dependencies::install_deps;
use fs_extra::{self as fsx, dir as dirx};
use pico_args::Arguments;
use semver::Version;
use std::{
    env,
    error::Error,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use version::{bump_versions, bump_versions_nightly};

const HELP_STR: &str = r#"
cargo xtask
Developement actions for ALVR.

USAGE:
    cargo xtask <SUBCOMMAND> [FLAG] [ARGS]

SUBCOMMANDS:
    install-deps        Download and compile/install external dependencies
    build-server        Build server driver, then copy binaries to build folder
    build-client        Build client, then copy binaries to build folder
    publish             Build server and client in release mode, zip server and copy the pdb file.
    clean               Removes build folder
    kill-oculus         Kill all Oculus processes
    bump-versions       Bump server and/or client package versions

FLAGS:
    --release           Optimized build without debug info. Used only for build subcommands
    --help              Print this text

ARGS:
    --client <VERSION>  Specify client version to set with the bump-versions subcommand
    --server <VERSION>  Specify server version to set with the bump-versions subcommand
"#;

type BResult<T = ()> = Result<T, Box<dyn Error>>;

struct Args {
    is_release: bool,
    server_version: Option<String>,
    client_version: Option<String>,
    nightly_version: bool,
}

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

fn run_with_args_in(workdir: &Path, cmd: &str, args: &[&str]) -> BResult {
    println!(
        "\n{}",
        args.iter().fold(String::from(cmd), |s, arg| s + " " + arg)
    );
    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .current_dir(workdir)
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

fn run_with_args(cmd: &str, args: &[&str]) -> BResult {
    run_with_args_in(&env::current_dir().unwrap(), cmd, args)
}

fn run(cmd: &str) -> BResult {
    let cmd_args = cmd.split_whitespace().collect::<Vec<_>>();
    run_with_args(cmd_args[0], &cmd_args[1..])
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

pub fn reset_server_build_folder() {
    fs::remove_dir_all(&server_build_dir()).ok();
    fs::create_dir_all(&server_build_dir()).unwrap();

    // get all file and folder paths at depth 1, excluded template root (at index 0)
    let dir_content =
        dirx::get_dir_content2("server_release_template", &dirx::DirOptions { depth: 1 }).unwrap();
    let items: Vec<&String> = dir_content.directories[1..]
        .iter()
        .chain(dir_content.files.iter())
        .collect();

    fsx::copy_items(&items, server_build_dir(), &dirx::CopyOptions::new()).unwrap();
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
            zip.start_file(name.to_string_lossy(), <_>::default())?;
            let mut f = fs::File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            println!("adding dir {:?} as {:?} ...", path, name);
            zip.add_directory(name.to_string_lossy(), <_>::default())?;
        }
    }

    Ok(())
}

pub fn build_server(is_release: bool, is_nightly: bool, fetch_crates: bool) {
    let build_type = if is_release { "release" } else { "debug" };
    let build_flag = if is_release { "--release" } else { "" };

    let target_dir = target_dir();
    let artifacts_dir = target_dir.join(build_type);
    let driver_dst_dir = server_build_dir().join("bin").join(STEAMVR_OS_DIR_NAME);
    let swresample_dir = workspace_dir().join("alvr/server/cpp/libswresample/lib");
    let openvr_api_dir = workspace_dir().join("alvr/server/cpp/openvr/lib");

    reset_server_build_folder();
    fs::create_dir_all(&driver_dst_dir).unwrap();

    if fetch_crates {
        run("cargo update").unwrap();
    }

    if is_nightly {
        env::set_current_dir(&workspace_dir().join("alvr/server")).unwrap();
        run(&format!(
            "cargo build {} --features alvr_common/nightly",
            build_flag
        ))
        .unwrap();
        env::set_current_dir(&workspace_dir().join("alvr/launcher")).unwrap();
        run(&format!(
            "cargo build {} --features alvr_common/nightly",
            build_flag
        ))
        .unwrap();
        env::set_current_dir(&workspace_dir()).unwrap();
    } else {
        run(&format!(
            "cargo build -p alvr_server -p alvr_launcher {}",
            build_flag
        ))
        .unwrap();
    }
    fs::copy(
        artifacts_dir.join(dynlib_fname("alvr_server")),
        driver_dst_dir.join(DRIVER_FNAME),
    )
    .unwrap();
    fs::copy(
        swresample_dir.join("avutil-56.dll"),
        driver_dst_dir.join("avutil-56.dll"),
    )
    .unwrap();
    fs::copy(
        swresample_dir.join("swresample-3.dll"),
        driver_dst_dir.join("swresample-3.dll"),
    )
    .unwrap();
    fs::copy(
        openvr_api_dir.join("openvr_api.dll"),
        driver_dst_dir.join("openvr_api.dll"),
    )
    .unwrap();
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_launcher")),
        server_build_dir().join(exec_fname("ALVR Launcher")),
    )
    .unwrap();

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
}

pub fn build_client(is_release: bool) {
    let build_type = if is_release { "release" } else { "debug" };
    let build_task = if is_release {
        "assembleRelease"
    } else {
        "assembleDebug"
    };

    let client_dir = workspace_dir().join("alvr/client/android");
    let command_name = if cfg!(not(windows)) {
        "gradlew"
    } else {
        "gradlew.bat"
    };

    fs::create_dir_all(&build_dir()).unwrap();

    env::set_current_dir(&client_dir).unwrap();
    run(&format!("{} {}", command_name, build_task)).unwrap();
    env::set_current_dir(workspace_dir()).unwrap();

    fs::copy(
        client_dir
            .join("app/build/outputs/apk")
            .join(build_type)
            .join(format!("app-{}.apk", build_type)),
        build_dir().join("alvr_client.apk"),
    )
    .unwrap();
}

fn download_vc_redist() {
    if ! Path::new("target/wix/VC_redist.x64.exe").is_file() {
        println!("Downloading Microsoft Visual C++ redistributable for bundling...");
        let mut vcredist = fs::File::create("target/wix/VC_redist.x64.exe").unwrap();
        reqwest::blocking::get("https://aka.ms/vs/16/release/vc_redist.x64.exe").unwrap().copy_to(&mut vcredist).unwrap();
    } else {
        println!("Found existing VC_redist.x64.exe - will use that.");
    }
}

fn build_installer(wix_path: &str) {
    let wix_path = PathBuf::from(wix_path).join("bin");
    let heat_cmd = wix_path.join("heat.exe");
    let candle_cmd = wix_path.join("candle.exe");
    let light_cmd = wix_path.join("light.exe");

    let mut version = Version::parse(&alvr_xtask::server_version()).unwrap();
    // Clear away build and prerelease version specifiers, MSI can have only dot-separated numbers.
    version.pre.clear();
    version.build.clear();

    run_with_args(
        &heat_cmd.to_string_lossy(),
        &[
            "dir",
            "build\\alvr_server_windows",
            "-ag",
            "-sreg",
            "-srd",
            "-dr",
            "APPLICATIONFOLDER",
            "-cg",
            "BuildFiles",
            "-var",
            "var.BuildRoot",
            "-o",
            "target\\wix\\harvested.wxs",
        ],
    )
    .unwrap();

    run_with_args(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            &format!("-dVersion={}", version),
            "wix\\main.wxs",
            "target\\wix\\harvested.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    run_with_args(
        &light_cmd.to_string_lossy(),
        &[
            "target\\wix\\main.wixobj",
            "target\\wix\\harvested.wixobj",
            "-ext",
            "WixUIExtension",
            "-ext",
            "WixUtilExtension",
            "-o",
            "target\\wix\\alvr.msi",
        ],
    )
    .unwrap();

    download_vc_redist();

    // Build the bundle including ALVR and vc_redist.
    run_with_args(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "wix\\bundle.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    run_with_args(
        &light_cmd.to_string_lossy(),
        &[
            "target\\wix\\bundle.wixobj",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "-o",
            &format!("build\\ALVR_Installer_v{}.exe", version),
        ],
    )
    .unwrap();
}

pub fn build_publish(is_nightly: bool) {
    build_server(true, is_nightly, false);
    build_client(true);
    zip_dir(&server_build_dir()).unwrap();

    if cfg!(windows) {
        fs::copy(
            target_dir().join("release").join("alvr_server.pdb"),
            build_dir().join("alvr_server.pdb"),
        )
        .unwrap();

        if let Some(wix_evar) = env::vars().find(|v| v.0 == "WIX") {
            println!("Found WiX, will build installer.");

            build_installer(&wix_evar.1);
        } else {
            println!("No WiX toolset installation found, skipping installer.");
        }
    }
}

// Avoid Oculus link popups when debugging the client
pub fn kill_oculus_processes() {
    runas::Command::new("taskkill")
        .args(&["/F", "/IM", "OVR*", "/T"])
        .status()
        .ok();
}

fn main() {
    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{}", HELP_STR);
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let args_values = Args {
            is_release: args.contains("--release"),
            server_version: args.opt_value_from_str("--server").unwrap(),
            client_version: args.opt_value_from_str("--client").unwrap(),
            nightly_version: args.contains("--nightly"),
        };
        if args.finish().is_ok() {
            match subcommand.as_str() {
                "install-deps" => install_deps(),
                "build-server" => build_server(args_values.is_release, false, true),
                "build-client" => build_client(args_values.is_release),
                "publish" => build_publish(args_values.nightly_version),
                "clean" => remove_build_dir(),
                "kill-oculus" => kill_oculus_processes(),
                "bump-versions" => {
                    if args_values.nightly_version {
                        bump_versions_nightly()
                    } else {
                        bump_versions(args_values.server_version, args_values.client_version)
                    }
                }
                _ => {
                    println!("\nUnrecognized subcommand.");
                    println!("{}", HELP_STR);
                    return;
                }
            }
        } else {
            println!("\nWrong arguments.");
            println!("{}", HELP_STR);
            return;
        }
    } else {
        println!("\nMissing subcommand.");
        println!("{}", HELP_STR);
        return;
    }

    println!("\nDone\n");
}
