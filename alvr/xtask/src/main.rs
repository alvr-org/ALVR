mod command;
mod dependencies;
mod version;

use fs_extra::{self as fsx, dir as dirx};
use pico_args::Arguments;
use semver::Version;
use std::{
    env,
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const HELP_STR: &str = r#"
cargo xtask
Developement actions for ALVR.

USAGE:
    cargo xtask <SUBCOMMAND> [FLAG] [ARGS]

SUBCOMMANDS:
    build-windows-deps  Download and compile external dependencies for Windows
    build-android-deps  Download and compile external dependencies for Android
    build-server        Build server driver, then copy binaries to build folder
    build-client        Build client, then copy binaries to build folder
    build-ffmpeg-linux  Build FFmpeg with VAAPI and Vulkan support. Only for CI
    publish-server      Build server in release mode, make portable version and installer
    publish-client      Build client for all headsets
    clean               Removes build folder
    kill-oculus         Kill all Oculus processes
    bump-versions       Bump server and client package versions
    clippy              Show warnings for selected clippy lints

FLAGS:
    --fetch             Update crates with "cargo update". Used only for build subcommands
    --release           Optimized build without debug info. Used only for build subcommands
    --nightly           Bump versions to nightly and build. Used only for publish subcommand
    --oculus-quest      Oculus Quest build. Used only for build-client subcommand
    --oculus-go         Oculus Go build. Used only for build-client subcommand
    --help              Print this text

ARGS:
    --version <VERSION>     Specify version to set with the bump-versions subcommand
"#;

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
    let dir_content = dirx::get_dir_content2(
        workspace_dir()
            .join("alvr")
            .join("xtask")
            .join("server_release_template"),
        &dirx::DirOptions { depth: 1 },
    )
    .unwrap();
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
            zip.write_all(&fs::read(path).unwrap())?;
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            println!("adding dir {:?} as {:?} ...", path, name);
            zip.add_directory(name.to_string_lossy(), <_>::default())?;
        }
    }

    Ok(())
}

pub fn build_server(is_release: bool, is_nightly: bool, fetch_crates: bool, new_dashboard: bool) {
    let build_type = if is_release { "release" } else { "debug" };
    let build_flag = if is_release { "--release" } else { "" };

    let target_dir = target_dir();
    let artifacts_dir = target_dir.join(build_type);
    let driver_dst_dir = server_build_dir().join("bin").join(STEAMVR_OS_DIR_NAME);

    reset_server_build_folder();
    fs::create_dir_all(&driver_dst_dir).unwrap();

    if fetch_crates {
        command::run("cargo update").unwrap();
    }

    if is_nightly {
        command::run_in(
            &workspace_dir().join("alvr/server"),
            &format!("cargo build {} --features alvr_common/nightly", build_flag),
        )
        .unwrap();
        command::run_in(
            &workspace_dir().join("alvr/launcher"),
            &format!("cargo build {} --features alvr_common/nightly", build_flag),
        )
        .unwrap();
    } else if new_dashboard {
        command::run_in(
            &workspace_dir().join("alvr/server"),
            &format!(
                "cargo build --no-default-features {} {}",
                " --features new_dashboard --features alvr_common/new_dashboard", build_flag
            ),
        )
        .unwrap();
        command::run_in(
            &workspace_dir().join("alvr/launcher"),
            &format!(
                "cargo build --no-default-features {} {}",
                "--features alvr_common/new_dashboard", build_flag
            ),
        )
        .unwrap();
    } else {
        command::run(&format!(
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

    if cfg!(windows) {
        let dir_content = dirx::get_dir_content("alvr/server/cpp/bin/windows").unwrap();
        fsx::copy_items(
            &dir_content.files,
            driver_dst_dir,
            &dirx::CopyOptions::new(),
        )
        .unwrap();
    }

    if new_dashboard {
        command::run_in(
            &workspace_dir().join("alvr/dashboard"),
            &format!(
                "npm install && npx webpack --mode {} --output-path=../../build/{}/dashboard",
                if is_release {
                    "production"
                } else {
                    "development"
                },
                SERVER_BUILD_DIR_NAME,
            ),
        )
        .unwrap()
    } else {
        let dir_content =
            dirx::get_dir_content2("alvr/legacy_dashboard", &dirx::DirOptions { depth: 1 })
                .unwrap();
        let items: Vec<&String> = dir_content.directories[1..]
            .iter()
            .chain(dir_content.files.iter())
            .collect();

        let destination = server_build_dir().join("dashboard");
        fs::create_dir_all(&destination).unwrap();
        fsx::copy_items(&items, destination, &dirx::CopyOptions::new()).unwrap();
    }

    fs::copy(
        artifacts_dir.join(exec_fname("alvr_launcher")),
        server_build_dir().join(exec_fname("ALVR Launcher")),
    )
    .unwrap();
}

pub fn build_client(is_release: bool, is_nightly: bool, for_oculus_go: bool, new_dashboard: bool) {
    let headset_name = if for_oculus_go {
        "oculus_go"
    } else {
        "oculus_quest"
    };

    let headset_type = if for_oculus_go {
        "OculusGo"
    } else {
        "OculusQuest"
    };
    let package_type = if is_nightly { "Nightly" } else { "Stable" };
    let build_type = &*format!(
        "{}{}",
        if is_release { "release" } else { "debug" },
        if new_dashboard { "NewDashboard" } else { "" }
    );

    let build_task = format!("assemble{}{}{}", headset_type, package_type, build_type);

    let client_dir = workspace_dir().join("alvr/client/android");
    let command_name = if cfg!(not(windows)) {
        "gradlew"
    } else {
        "gradlew.bat"
    };

    let artifact_name = format!("alvr_client_{}", headset_name);
    fs::create_dir_all(&build_dir().join(&artifact_name)).unwrap();

    env::set_current_dir(&client_dir).unwrap();
    command::run(&format!("{} {}", command_name, build_task)).unwrap();
    env::set_current_dir(workspace_dir()).unwrap();

    fs::copy(
        client_dir
            .join("app/build/outputs/apk")
            .join(format!("{}{}", headset_type, package_type))
            .join(build_type)
            .join(format!(
                "app-{}-{}-{}.apk",
                headset_type, package_type, build_type
            )),
        build_dir()
            .join(&artifact_name)
            .join(format!("{}.apk", artifact_name)),
    )
    .unwrap();
}

fn build_installer(wix_path: &str) {
    let wix_path = PathBuf::from(wix_path).join("bin");
    let heat_cmd = wix_path.join("heat.exe");
    let candle_cmd = wix_path.join("candle.exe");
    let light_cmd = wix_path.join("light.exe");

    let mut version = Version::parse(&version::version()).unwrap();
    // Clear away build and prerelease version specifiers, MSI can have only dot-separated numbers.
    version.pre.clear();
    version.build.clear();

    command::run_without_shell(
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

    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            &format!("-dVersion={}", version),
            "alvr\\xtask\\wix\\main.wxs",
            "target\\wix\\harvested.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    command::run_without_shell(
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

    // Build the bundle including ALVR and vc_redist.
    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "alvr\\xtask\\wix\\bundle.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    command::run_without_shell(
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

pub fn publish_server(is_nightly: bool) {
    build_server(true, is_nightly, false, false);
    zip_dir(&server_build_dir()).unwrap();

    if cfg!(windows) {
        if is_nightly {
            fs::copy(
                target_dir().join("release").join("alvr_server.pdb"),
                build_dir().join("alvr_server.pdb"),
            )
            .unwrap();
        }

        if let Some(wix_evar) = env::vars().find(|v| v.0 == "WIX") {
            println!("Found WiX, will build installer.");

            build_installer(&wix_evar.1);
        } else {
            println!("No WiX toolset installation found, skipping installer.");
        }
    }
}

pub fn publish_client(is_nightly: bool) {
    build_client(!is_nightly, is_nightly, false, false);
    build_client(!is_nightly, is_nightly, true, false);
}

// Avoid Oculus link popups when debugging the client
pub fn kill_oculus_processes() {
    runas::Command::new("taskkill")
        .args(&["/F", "/IM", "OVR*", "/T"])
        .status()
        .ok();
}

fn clippy() {
    command::run(&format!(
        "cargo clippy {} -- {} {} {} {} {} {} {} {} {} {} {}",
        "-p alvr_xtask -p alvr_common -p alvr_launcher -p alvr_dashboard", // todo: add more crates when they compile correctly
        "-W clippy::clone_on_ref_ptr -W clippy::create_dir -W clippy::dbg_macro",
        "-W clippy::decimal_literal_representation -W clippy::else_if_without_else",
        "-W clippy::exit -W clippy::expect_used -W clippy::filetype_is_file",
        "-W clippy::float_cmp_const -W clippy::get_unwrap -W clippy::let_underscore_must_use",
        "-W clippy::lossy_float_literal -W clippy::map_err_ignore -W clippy::mem_forget",
        "-W clippy::multiple_inherent_impl -W clippy::print_stderr -W clippy::print_stderr",
        "-W clippy::rc_buffer -W clippy::rest_pat_in_fully_bound_structs -W clippy::str_to_string",
        "-W clippy::string_to_string -W clippy::todo -W clippy::unimplemented",
        "-W clippy::unneeded_field_pattern -W clippy::unwrap_in_result",
        "-W clippy::verbose_file_reads -W clippy::wildcard_enum_match_arm",
        "-W clippy::wrong_pub_self_convention"
    ))
    .unwrap();
}

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{}", HELP_STR);
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let fetch = args.contains("--fetch");
        let is_release = args.contains("--release");
        let version: Option<String> = args.opt_value_from_str("--version").unwrap();
        let is_nightly = args.contains("--nightly");
        let for_oculus_quest = args.contains("--oculus-quest");
        let for_oculus_go = args.contains("--oculus-go");
        let new_dashboard = args.contains("--new-dashboard");

        if args.finish().is_empty() {
            match subcommand.as_str() {
                "build-windows-deps" => dependencies::build_deps("windows"),
                "build-android-deps" => dependencies::build_deps("android"),
                "build-server" => build_server(is_release, false, fetch, new_dashboard),
                "build-client" => {
                    if (for_oculus_quest && for_oculus_go) || (!for_oculus_quest && !for_oculus_go)
                    {
                        build_client(is_release, false, false, new_dashboard);
                        build_client(is_release, false, true, new_dashboard);
                    } else {
                        build_client(is_release, false, for_oculus_go, new_dashboard);
                    }
                }
                "build-ffmpeg-linux" => dependencies::build_ffmpeg_linux(),
                "publish-server" => publish_server(is_nightly),
                "publish-client" => publish_client(is_nightly),
                "clean" => remove_build_dir(),
                "kill-oculus" => kill_oculus_processes(),
                "bump-versions" => version::bump_version(version.as_deref(), is_nightly),
                "clippy" => clippy(),
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
