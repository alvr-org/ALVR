mod build;
mod command;
mod dependencies;
mod packaging;
mod version;

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
    prepare-deps        Download and compile server and client external dependencies
    build-server        Build server driver, then copy binaries to build folder
    build-client        Build client, then copy binaries to build folder
    package-server      Build server in release mode, make portable version and installer
    clean               Removes all build artifacts and dependencies.
    bump                Bump server and client package versions
    clippy              Show warnings for selected clippy lints
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

// Crates at "alvr/" level that are prefixed with "alvr_"
pub fn crate_dir_names() -> Vec<String> {
    let sh = Shell::new().unwrap();

    sh.read_dir(afs::workspace_dir().join("alvr"))
        .unwrap()
        .into_iter()
        .map(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .as_ref()
                .to_owned()
        })
        .collect()
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
    let crate_flags = crate_dir_names()
        .into_iter()
        .filter(|name| name != "client" && name != "vulkan_layer")
        .flat_map(|name| ["-p".into(), format!("alvr_{name}")]);

    // lints updated for Rust 1.59
    let restriction_lints = [
        // "allow_attributes_without_reason", // Rust 1.61
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
        "self_named_module_files",
        "str_to_string",
        "string_slice",
        "string_to_string",
        "try_err",
        "unnecessary_self_imports",
        "unneeded_field_pattern",
        "unseparated_literal_suffix",
        "verbose_file_reads",
        "wildcard_enum_match_arm",
    ];
    let pedantic_lints = [
        // "borrow_as_ptr", // Rust 1.60
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
    cmd!(sh, "cargo clippy {crate_flags...} -- {flags...}")
        .run()
        .unwrap();
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
                                dependencies::build_android_deps(for_ci)
                            }
                            _ => panic!("Unrecognized platform."),
                        }
                    } else {
                        if cfg!(windows) {
                            dependencies::prepare_windows_deps(for_ci);
                        } else if cfg!(target_os = "linux") {
                            dependencies::build_ffmpeg_linux(!no_nvidia);
                        }

                        dependencies::build_android_deps(for_ci);
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
