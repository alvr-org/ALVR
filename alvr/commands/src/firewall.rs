use std::{env, fs, path::Path, process::Command};

fn netsh_add_rule_command_string(rule_name: &str, program_path: &Path) -> String {
    format!(
        "netsh advfirewall firewall add rule name=\"{}\" dir=in program=\"{}\" action=allow",
        rule_name,
        program_path.to_string_lossy()
    )
}

fn netsh_delete_rule_command_string(rule_name: &str) -> String {
    format!("netsh advfirewall firewall delete rule name=\"{rule_name}\"")
}

// Errors:
// 1: firewall rule is already set
// 126: pkexec request dismissed
// other: command failed
pub fn firewall_rules(add: bool) -> Result<(), i32> {
    let exit_status = if cfg!(target_os = "linux") {
        let action = if add { "add" } else { "remove" };
        // run as normal user since we use pkexec to sudo
        Command::new("bash")
            .arg("/usr/libexec/alvr/alvr_fw_config.sh")
            .arg(action)
            .status()
            .map_err(|_| -1)?
    } else {
        let script_path = env::temp_dir().join("alvr_firewall_rules.bat");
        let firewall_rules_script_content = if add {
            format!(
                "{}\n{}",
                netsh_add_rule_command_string(
                    "SteamVR ALVR vrserver",
                    &crate::steamvr_root_dir()
                        .map_err(|_| -1)?
                        .join("bin")
                        .join("win64")
                        .join("vrserver.exe")
                ),
                netsh_add_rule_command_string(
                    "SteamVR ALVR vrserver",
                    &crate::steamvr_root_dir()
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

        // run with admin privileges
        runas::Command::new(script_path)
            .gui(true) // UAC, if available
            .status()
            .map_err(|_| -1)?
    };

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap())
    }
}
