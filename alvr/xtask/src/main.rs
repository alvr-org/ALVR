mod build;
mod command;
mod dependencies;
mod packaging;
mod version;

use alvr_filesystem as afs;
use pico_args::Arguments;
use std::{env, fs, time::Instant};

const HELP_STR: &str = r#"
cargo xtask
Developement actions for ALVR.

USAGE:
    cargo xtask <SUBCOMMAND> [FLAG] [ARGS]

SUBCOMMANDS:
    prepare-deps        Download and compile server and client external dependencies
    build-server        Build server driver, then copy binaries to build folder
    build-client        Build client, then copy binaries to build folder
    package-server      Build server in release mode, make portable version and installer
    clean               Removes all build artifacts and dependencies.
    bump                Bump server and client package versions
    clippy              Show warnings for selected clippy lints
    prettier            Format JS and CSS files with prettier; Requires Node.js and NPM.
    kill-oculus         Kill all Oculus processes

FLAGS:
    --help              Print this text
    --no-nvidia         Disables nVidia support on Linux. For prepare-deps subcommand
    --release           Optimized build without debug info. For build subcommands
    --gpl               Bundle GPL libraries. For build subcommands
    --experiments       Build unfinished features. For build subcommands
    --nightly           Append nightly tag to versions. For bump subcommand
    --ci                Do some CI related tweaks. Depends on the other flags and subcommand

ARGS:
    --platform <NAME>   Name of the platform (operative system or hardware name). snake_case
    --version <VERSION> Specify version to set with the bump-versions subcommand
    --root <PATH>       Installation root. By default no root is set and paths are calculated using
                        relative paths, which requires conforming to FHS on Linux.
"#;

pub fn clean() {
    fs::remove_dir_all(afs::build_dir()).ok();
    fs::remove_dir_all(afs::deps_dir()).ok();
    if afs::target_dir() == afs::workspace_dir().join("target") {
        // Detete target folder only if in the local wokspace!
        fs::remove_dir_all(afs::target_dir()).ok();
    }
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

fn main() {
    let begin_time = Instant::now();

    env::set_var("RUST_BACKTRACE", "1");

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{HELP_STR}");
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let no_nvidia = args.contains("--no-nvidia");
        let is_release = args.contains("--release");
        let gpl = args.contains("--gpl");
        let experiments = args.contains("--experiments");
        let is_nightly = args.contains("--nightly");
        let for_ci = args.contains("--ci");

        let platform: Option<String> = args.opt_value_from_str("--platform").unwrap();
        let version: Option<String> = args.opt_value_from_str("--version").unwrap();
        let root: Option<String> = args.opt_value_from_str("--root").unwrap();

        if args.finish().is_empty() {
            match subcommand.as_str() {
                "prepare-deps" => {
                    if let Some(platform) = platform {
                        match platform.as_str() {
                            "windows" => dependencies::prepare_windows_deps(for_ci),
                            "linux" => dependencies::build_ffmpeg_linux(!no_nvidia),
                            "android" | "oculus_quest" | "oculus_go" => {
                                dependencies::build_android_deps()
                            }
                            _ => panic!("Unrecognized platform."),
                        }
                    } else {
                        if cfg!(windows) {
                            dependencies::prepare_windows_deps(for_ci);
                        } else if cfg!(target_os = "linux") {
                            dependencies::build_ffmpeg_linux(!no_nvidia);
                        }

                        dependencies::build_android_deps();
                    }
                }
                "build-server" => build::build_server(is_release, gpl, None, false, experiments),
                "build-client" => {
                    if let Some(platform) = platform {
                        build::build_client(is_release, &platform);
                    } else {
                        build::build_client(is_release, "oculus_quest");
                        build::build_client(is_release, "oculus_go");
                    }
                }
                "package-server" => packaging::package_server(root, gpl),
                "package-client" => {
                    if let Some(platform) = platform {
                        build::build_client(true, &platform);
                    } else {
                        build::build_client(true, "oculus_quest");
                        build::build_client(true, "oculus_go");
                    }
                }
                "clean" => clean(),
                "bump" => version::bump_version(version, is_nightly),
                "clippy" => clippy(),
                "prettier" => prettier(),
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
