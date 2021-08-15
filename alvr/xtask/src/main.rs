mod command;
mod dependencies;
mod version;

use fs_extra::{self as fsx, dir as dirx};
use pico_args::Arguments;
use std::{
    env,
    error::Error,
    fs,
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
    prettier            Format JS and CSS files with prettier; Requires Node.js and NPM.

FLAGS:
    --fetch             Update crates with "cargo update". Used only for build subcommands
    --release           Optimized build without debug info. Used only for build subcommands
    --test              Build testing utilities and unfinished features
    --nightly           Bump versions to nightly and build. Used only for publish subcommand
    --oculus-quest      Oculus Quest build. Used only for build-client subcommand
    --oculus-go         Oculus Go build. Used only for build-client subcommand
    --bundle-ffmpeg     Bundle ffmpeg libraries. Only used for build-server subcommand on Linux
    --help              Print this text

ARGS:
    --version <VERSION>     Specify version to set with the bump-versions subcommand
"#;

type BResult<T = ()> = Result<T, Box<dyn Error>>;

#[cfg(target_os = "linux")]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_linux";
#[cfg(windows)]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_windows";
#[cfg(target_os = "macos")]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_macos";

#[cfg(not(windows))]
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
#[cfg(target_os = "macos")]
fn dynlib_fname(name: &str) -> String {
    format!("lib{}.dylib", name)
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

pub fn build_server(is_release: bool, test: bool, fetch_crates: bool, bundle_ffmpeg: bool) {
    let build_type = if is_release { "release" } else { "debug" };
    let build_flag = if is_release { "--release" } else { "" };

    let mut server_features: Vec<&str> = vec![];
    let mut launcher_features: Vec<&str> = vec![];

    if bundle_ffmpeg {
        server_features.push("bundled_ffmpeg");
    }
    if server_features.is_empty() {
        server_features.push("default")
    }
    if launcher_features.is_empty() {
        launcher_features.push("default")
    }

    let target_dir = target_dir();
    let artifacts_dir = target_dir.join(build_type);
    let driver_dst_dir = server_build_dir().join(
        alvr_filesystem_layout::LAYOUT
            .openvr_driver_lib()
            .parent()
            .unwrap(),
    );

    if fetch_crates {
        command::run("cargo update").unwrap();
    }

    fs::remove_dir_all(&server_build_dir()).ok();
    fs::create_dir_all(&server_build_dir()).unwrap();
    fs::create_dir_all(&driver_dst_dir).unwrap();
    fs::create_dir_all(
        server_build_dir().join(
            alvr_filesystem_layout::LAYOUT
                .launcher_exe
                .parent()
                .unwrap(),
        ),
    )
    .unwrap();

    let mut copy_options = dirx::CopyOptions::new();
    copy_options.copy_inside = true;
    fsx::copy_items(
        &["alvr/xtask/resources/presets"],
        server_build_dir().join(&alvr_filesystem_layout::LAYOUT.presets_dir),
        &copy_options,
    )
    .expect("copy presets");

    if bundle_ffmpeg {
        let ffmpeg_path = dependencies::build_ffmpeg_linux();
        let lib_dir = server_build_dir().join("lib64").join("alvr");
        fs::create_dir_all(lib_dir.clone()).unwrap();
        for lib in walkdir::WalkDir::new(ffmpeg_path)
            .into_iter()
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.into_path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().contains(".so."))
        {
            fs::copy(lib.clone(), lib_dir.join(lib.file_name().unwrap())).unwrap();
        }
    }

    if cfg!(target_os = "linux") {
        command::run_in(
            &workspace_dir().join("alvr/vrcompositor-wrapper"),
            &format!("cargo build {}", build_flag),
        )
        .unwrap();
        fs::create_dir_all(
            server_build_dir().join(
                alvr_filesystem_layout::LAYOUT
                    .vrcompositor_wrapper
                    .parent()
                    .unwrap(),
            ),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join("vrcompositor-wrapper"),
            server_build_dir().join(&alvr_filesystem_layout::LAYOUT.vrcompositor_wrapper),
        )
        .unwrap();
    }

    if cfg!(not(target_os = "macos")) {
        command::run_in(
            &workspace_dir().join("alvr/server"),
            &format!(
                "cargo build {} --no-default-features --features {}",
                build_flag,
                server_features.join(",")
            ),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join(dynlib_fname("alvr_server")),
            server_build_dir().join(alvr_filesystem_layout::LAYOUT.openvr_driver_lib()),
        )
        .unwrap();
    }
    command::run_in(
        &workspace_dir().join("alvr/launcher"),
        &format!(
            "cargo build {} --no-default-features --features {}",
            build_flag,
            launcher_features.join(",")
        ),
    )
    .unwrap();
    fs::copy(
        artifacts_dir.join(exec_fname("alvr_launcher")),
        server_build_dir().join(&alvr_filesystem_layout::LAYOUT.launcher_exe),
    )
    .unwrap();
    if test {
        let dir_content =
            dirx::get_dir_content2("alvr/gui/languages", &dirx::DirOptions { depth: 1 }).unwrap();
        let items: Vec<&String> = dir_content.directories[1..]
            .iter()
            .chain(dir_content.files.iter())
            .collect();

        let destination = server_build_dir().join("languages");
        fs::create_dir_all(&destination).unwrap();
        fsx::copy_items(&items, destination, &dirx::CopyOptions::new()).unwrap();

        command::run_in(
            &workspace_dir().join("alvr/egui_dashboard"),
            &format!("cargo build {}", build_flag),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join(exec_fname("alvr_egui_dashboard")),
            server_build_dir().join(exec_fname("alvr_egui_dashboard")),
        )
        .unwrap();
    }

    fs::copy(
        std::path::Path::new("alvr/xtask/resources/driver.vrdrivermanifest"),
        server_build_dir().join(alvr_filesystem_layout::LAYOUT.openvr_driver_manifest()),
    )
    .expect("copy openVR driver manifest");

    if cfg!(windows) {
        let dir_content = dirx::get_dir_content("alvr/server/cpp/bin/windows").unwrap();
        fsx::copy_items(
            &dir_content.files,
            driver_dst_dir,
            &dirx::CopyOptions::new(),
        )
        .unwrap();
    }

    let dir_content =
        dirx::get_dir_content2("alvr/dashboard", &dirx::DirOptions { depth: 1 }).unwrap();
    let items: Vec<&String> = dir_content.directories[1..]
        .iter()
        .chain(dir_content.files.iter())
        .collect();

    let destination =
        server_build_dir().join(&alvr_filesystem_layout::LAYOUT.dashboard_resources_dir);
    fs::create_dir_all(&destination).unwrap();
    fsx::copy_items(&items, destination, &dirx::CopyOptions::new()).unwrap();

    if cfg!(target_os = "linux") {
        command::run_in(
            &workspace_dir().join("alvr/vulkan-layer"),
            &format!("cargo build {}", build_flag),
        )
        .unwrap();

        let lib_dir = server_build_dir().join("lib64");
        let manifest_dir = server_build_dir().join("share/vulkan/explicit_layer.d");

        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(&lib_dir).unwrap();
        fs::copy(
            workspace_dir().join("alvr/vulkan-layer/layer/alvr_x86_64.json"),
            manifest_dir.join("alvr_x86_64.json"),
        )
        .unwrap();
        fs::copy(
            artifacts_dir.join(dynlib_fname("alvr_vulkan_layer")),
            lib_dir.join(dynlib_fname("alvr_vulkan_layer")),
        )
        .unwrap();
    }
}

pub fn build_client(is_release: bool, is_nightly: bool, for_oculus_go: bool) {
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
    let build_type = if is_release { "release" } else { "debug" };

    let build_task = format!("assemble{}{}{}", headset_type, package_type, build_type);

    let client_dir = workspace_dir().join("alvr/client/android");
    let command_name = if cfg!(not(windows)) {
        "./gradlew"
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

    // Clear away build and prerelease version specifiers, MSI can have only dot-separated numbers.
    let mut version = version::version();
    if let Some(idx) = version.find('-') {
        version = version[..idx].to_owned();
    }
    if let Some(idx) = version.find('+') {
        version = version[..idx].to_owned();
    }

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
    build_server(true, false, false, false);

    // Add licenses
    let licenses_dir = server_build_dir().join("licenses");
    fs::create_dir_all(&licenses_dir).unwrap();
    fs::copy(
        workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();
    command::run("cargo install cargo-about").unwrap();
    command::run(&format!(
        "cargo about generate {} > {}",
        workspace_dir()
            .join("alvr")
            .join("xtask")
            .join("licenses_template.hbs")
            .to_string_lossy(),
        licenses_dir.join("dependencies.html").to_string_lossy()
    ))
    .unwrap();
    fs::copy(
        workspace_dir()
            .join("alvr")
            .join("server")
            .join("LICENSE-Valve"),
        licenses_dir.join("Valve.txt"),
    )
    .unwrap();

    command::zip(&server_build_dir()).unwrap();

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
    build_client(!is_nightly, is_nightly, false);
    build_client(!is_nightly, is_nightly, true);
}

// Avoid Oculus link popups when debugging the client
pub fn kill_oculus_processes() {
    command::run_without_shell(
        "powershell",
        &[
            "Start-Process",
            "taskkill",
            "-ArgumentList",
            "\"/F /IM OVR* /T\"",
            "-Verb",
            "runAs",
        ],
    )
    .unwrap();
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

fn prettier() {
    command::run("npx -p prettier@2.2.1 prettier --config alvr/xtask/.prettierrc --write '**/*[!.min].{css,js}'").unwrap();
}

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{}", HELP_STR);
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let fetch = args.contains("--fetch");
        let is_release = args.contains("--release");
        let test = args.contains("--test");
        let version: Option<String> = args.opt_value_from_str("--version").unwrap();
        let is_nightly = args.contains("--nightly");
        let for_oculus_quest = args.contains("--oculus-quest");
        let for_oculus_go = args.contains("--oculus-go");
        let bundle_ffmpeg = args.contains("--bundle-ffmpeg");

        if args.finish().is_empty() {
            match subcommand.as_str() {
                "build-windows-deps" => dependencies::build_deps("windows"),
                "build-android-deps" => dependencies::build_deps("android"),
                "build-server" => build_server(is_release, test, fetch, bundle_ffmpeg),
                "build-client" => {
                    if (for_oculus_quest && for_oculus_go) || (!for_oculus_quest && !for_oculus_go)
                    {
                        build_client(is_release, false, false);
                        build_client(is_release, false, true);
                    } else {
                        build_client(is_release, false, for_oculus_go);
                    }
                }
                "build-ffmpeg-linux" => {
                    dependencies::build_ffmpeg_linux();
                }
                "publish-server" => publish_server(is_nightly),
                "publish-client" => publish_client(is_nightly),
                "clean" => remove_build_dir(),
                "kill-oculus" => kill_oculus_processes(),
                "bump-versions" => version::bump_version(version, is_nightly),
                "clippy" => clippy(),
                "prettier" => prettier(),
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
