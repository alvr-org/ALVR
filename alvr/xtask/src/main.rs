use alvr_xtask::*;
use pico_args::Arguments;

fn ok_or_exit<T, E: std::fmt::Display>(res: Result<T, E>) {
    use std::process::exit;

    if let Err(e) = res {
        #[cfg(not(windows))]
        {
            use termion::color::*;
            println!("\n{}Error: {}{}", Fg(Red), e, Fg(Reset));
        }
        #[cfg(windows)]
        println!("{}", e);

        exit(1);
    }
}

fn print_help() {
    println!(
        r#"
cargo xtask
Developement actions for ALVR.

USAGE:
    cargo xtask <SUBCOMMAND> [FLAG]

SUBCOMMANDS:
    install-deps        Install toolchains and required cargo third-party subcommands
    build-server        Build server driver and GUI, then copy binaries to build folder
    build-client        Build client, then copy binaries to build folder
    build-all           'build-server' + 'build-client'
    add-firewall-rules  Add firewall rules for ALVR web server and SteamVR vrserver
    register-driver     Register ALVR driver in SteamVR
    clean               Removes build folder

FLAGS:
    --release           Optimized build without debug info. Used only for build subcommands
    --help              Print this text
"#
    );
}

fn main() {

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        print_help();
    } else if let Ok(Some(subcommand)) = args.subcommand() {
        let is_release = args.contains("--release");

        if args.finish().is_ok() {
            match subcommand.as_str() {
                "install-deps" => ok_or_exit(install_deps()),
                "build-server" => ok_or_exit(build_server(is_release)),
                "build-client" => ok_or_exit(build_client(is_release)),
                "build-all" => {
                    ok_or_exit(build_server(is_release));
                    ok_or_exit(build_client(is_release));
                }
                "add-firewall-rules" => ok_or_exit(firewall_rules(&server_build_dir(), true)),
                "register-driver" => ok_or_exit(driver_registration(&server_build_dir(), true)),
                "clean" => remove_build_dir(),
                _ => {
                    println!("\nUnrecognized subcommand.");
                    print_help();
                    return;
                }
            }
        } else {
            println!("\nWrong arguments.");
            print_help();
            return;
        }
    } else {
        println!("\nMissing subcommand.");
        print_help();
        return;
    }

    println!("\nDone\n");
}
