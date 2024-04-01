mod build;
mod command;
mod dependencies;
mod packaging;
mod version;

use crate::build::Profile;
use afs::Layout;
use alvr_filesystem as afs;
use pico_args::Arguments;
use std::{fs, time::Instant};
use xshell::{cmd, Shell};

const HELP_STR: &str = r#"
cargo xtask
Developement actions for ALVR.

USAGE:
    cargo xtask <SUBCOMMAND> [FLAG] [ARGS]

SUBCOMMANDS:
    prepare-deps        Download and compile streamer and client external dependencies
    build-streamer      Build streamer, then copy binaries to build folder
    build-launcher      Build launcher, then copy binaries to build folder
    build-server-lib    Build a C-ABI ALVR server library and header
    build-client        Build client, then copy binaries to build folder
    build-client-lib    Build a C-ABI ALVR client library and header
    run-streamer        Build streamer and then open the dashboard
    run-launcher        Build launcher and then open it
    package-streamer    Build streamer with distribution profile, make archive
    package-launcher    Build launcher in release mode, make portable and installer versions
    package-client-lib  Build client library then zip it
    clean               Removes all build artifacts and dependencies
    bump                Bump streamer and client package versions
    clippy              Show warnings for selected clippy lints
    kill-oculus         Kill all Oculus processes

FLAGS:
    --help              Print this text
    --keep-config       Preserve the configuration file between rebuilds (session.json)
    --no-nvidia         Disables nVidia support on Linux. For prepare-deps subcommand
    --release           Optimized build with less debug checks. For build subcommands
    --profiling         Enable Profiling
    --gpl               Bundle GPL libraries (FFmpeg). Only for Windows
    --appimage          Package as AppImage. For package-streamer subcommand
    --zsync             For --appimage, create .zsync update file and build AppImage with embedded update information. For package-streamer subcommand
    --nightly           Append nightly tag to versions. For bump subcommand
    --no-rebuild        Do not rebuild the streamer with run-streamer
    --ci                Do some CI related tweaks. Depends on the other flags and subcommand
    --no-stdcpp         Disable linking to libc++_shared with build-client-lib

ARGS:
    --platform <NAME>   Name of the platform (operative system or hardware name). snake_case
    --version <VERSION> Specify version to set with the bump-versions subcommand
    --root <PATH>       Installation root. By default no root is set and paths are calculated using
                        relative paths, which requires conforming to FHS on Linux.
"#;

pub fn run_streamer() {
    let sh = Shell::new().unwrap();

    let dashboard_exe = Layout::new(&afs::streamer_build_dir()).dashboard_exe();
    cmd!(sh, "{dashboard_exe}").run().unwrap();
}

pub fn run_launcher() {
    let sh = Shell::new().unwrap();

    let launcher_exe = afs::launcher_build_exe_path();
    cmd!(sh, "{launcher_exe}").run().unwrap();
}

pub fn clean() {
    fs::remove_dir_all(afs::build_dir()).ok();
    fs::remove_dir_all(afs::deps_dir()).ok();
    if afs::target_dir() == afs::workspace_dir().join("target") {
        // Detete target folder only if in the local wokspace!
        fs::remove_dir_all(afs::target_dir()).ok();
    }
}

fn clippy() {
    // lints updated for Rust 1.59
    let restriction_lints = [
        "allow_attributes_without_reason",
        "clone_on_ref_ptr",
        "create_dir",
        "decimal_literal_representation",
        "else_if_without_else",
        "expect_used",
        "float_cmp_const",
        "fn_to_numeric_cast_any",
        "get_unwrap",
        "if_then_some_else_none",
        "let_underscore_must_use",
        "lossy_float_literal",
        "mem_forget",
        "multiple_inherent_impl",
        "rest_pat_in_fully_bound_structs",
        // "self_named_module_files",
        "str_to_string",
        // "string_slice",
        "string_to_string",
        "try_err",
        "unnecessary_self_imports",
        "unneeded_field_pattern",
        "unseparated_literal_suffix",
        "verbose_file_reads",
        "wildcard_enum_match_arm",
    ];
    let pedantic_lints = [
        "borrow_as_ptr",
        "enum_glob_use",
        "explicit_deref_methods",
        "explicit_into_iter_loop",
        "explicit_iter_loop",
        "filter_map_next",
        "flat_map_option",
        "float_cmp",
        // todo: add more lints
    ];

    let flags = restriction_lints
        .into_iter()
        .chain(pedantic_lints)
        .flat_map(|name| ["-W".to_owned(), format!("clippy::{name}")]);

    let sh = Shell::new().unwrap();
    cmd!(sh, "cargo clippy -- {flags...}").run().unwrap();
}

// Avoid Oculus link popups when debugging the client
pub fn kill_oculus_processes() {
    let sh = Shell::new().unwrap();
    cmd!(
        sh,
        "powershell Start-Process taskkill -ArgumentList \"/F /IM OVR* /T\" -Verb runas"
    )
    .run()
    .unwrap();
}

fn main() {
    let begin_time = Instant::now();

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{HELP_STR}");
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let no_nvidia = args.contains("--no-nvidia");
        let is_release = args.contains("--release");
        let profile = if is_release {
            Profile::Release
        } else {
            Profile::Debug
        };
        let profiling = args.contains("--profiling");
        let gpl = args.contains("--gpl");
        let is_nightly = args.contains("--nightly");
        let no_rebuild = args.contains("--no-rebuild");
        let for_ci = args.contains("--ci");
        let keep_config = args.contains("--keep-config");
        let appimage = args.contains("--appimage");
        let zsync = args.contains("--zsync");
        let link_stdcpp = !args.contains("--no-stdcpp");

        let platform: Option<String> = args.opt_value_from_str("--platform").unwrap();
        let version: Option<String> = args.opt_value_from_str("--version").unwrap();
        let root: Option<String> = args.opt_value_from_str("--root").unwrap();

        if args.finish().is_empty() {
            match subcommand.as_str() {
                "prepare-deps" => {
                    if let Some(platform) = platform {
                        match platform.as_str() {
                            "windows" => dependencies::prepare_windows_deps(for_ci),
                            "linux" => dependencies::prepare_linux_deps(!no_nvidia),
                            "android" => dependencies::build_android_deps(for_ci),
                            _ => panic!("Unrecognized platform."),
                        }
                    } else {
                        if cfg!(windows) {
                            dependencies::prepare_windows_deps(for_ci);
                        } else if cfg!(target_os = "linux") {
                            dependencies::prepare_linux_deps(!no_nvidia);
                        }

                        dependencies::build_android_deps(for_ci);
                    }
                }
                "build-streamer" => {
                    build::build_streamer(profile, true, gpl, None, false, profiling, keep_config)
                }
                "build-launcher" => build::build_launcher(profile, true, false),
                "build-server-lib" => build::build_server_lib(profile, true, gpl, None, false),
                "build-client" => build::build_android_client(profile),
                "build-client-lib" => build::build_client_lib(profile, link_stdcpp),
                "run-streamer" => {
                    if !no_rebuild {
                        build::build_streamer(
                            profile,
                            true,
                            gpl,
                            None,
                            false,
                            profiling,
                            keep_config,
                        );
                    }
                    run_streamer();
                }
                "run-launcher" => {
                    if !no_rebuild {
                        build::build_launcher(profile, true, false);
                    }
                    run_launcher();
                }
                "package-streamer" => packaging::package_streamer(gpl, root, appimage, zsync),
                "package-launcher" => packaging::package_launcher(appimage),
                "package-client" => build::build_android_client(Profile::Distribution),
                "package-client-lib" => packaging::package_client_lib(link_stdcpp),
                "clean" => clean(),
                "bump" => version::bump_version(version, is_nightly),
                "clippy" => clippy(),
                "kill-oculus" => kill_oculus_processes(),
                _ => {
                    println!("\nUnrecognized subcommand.");
                    println!("{HELP_STR}");
                    return;
                }
            }
        } else {
            println!("\nWrong arguments.");
            println!("{HELP_STR}");
            return;
        }
    } else {
        println!("\nMissing subcommand.");
        println!("{HELP_STR}");
        return;
    }

    let elapsed_time = Instant::now() - begin_time;
    println!(
        "\nDone [{}m {}s]\n",
        elapsed_time.as_secs() / 60,
        elapsed_time.as_secs() % 60
    );
}
