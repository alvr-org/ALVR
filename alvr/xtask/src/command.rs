use alvr_common::prelude::*;
use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

pub fn run_as_shell_in(workdir: &Path, shell: &str, shell_flag: &str, cmd: &str) -> StrResult {
    println!("\n> {}", cmd);

    let output = trace_err!(trace_err!(Command::new(shell)
        .args(&[shell_flag, cmd])
        .stdout(Stdio::inherit())
        .current_dir(workdir)
        .spawn())?
    .wait_with_output())?;

    if output.status.success() {
        Ok(())
    } else {
        fmt_e!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

pub fn run_in(workdir: &Path, cmd: &str) -> StrResult {
    let shell = if cfg!(windows) { "cmd" } else { "bash" };
    let shell_flag = if cfg!(windows) { "/C" } else { "-c" };

    run_as_shell_in(workdir, shell, shell_flag, cmd)
}

pub fn run(cmd: &str) -> StrResult {
    run_in(&env::current_dir().unwrap(), cmd)
}

// Bash can be invoked on Windows if WSL is installed
pub fn run_as_bash_in(workdir: &Path, cmd: &str) -> StrResult {
    run_as_shell_in(workdir, "bash", "-c", cmd)
}

pub fn run_without_shell(cmd: &str, args: &[&str]) -> StrResult {
    println!(
        "\n> {}",
        args.iter().fold(String::from(cmd), |s, arg| s + " " + arg)
    );
    let output = trace_err!(trace_err!(Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .spawn())?
    .wait_with_output())?;

    if output.status.success() {
        Ok(())
    } else {
        fmt_e!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

pub fn zip(source: &Path) -> StrResult {
    if cfg!(windows) {
        run_without_shell(
            "powershell",
            &[
                "Compress-Archive",
                &source.to_string_lossy(),
                &format!("{}.zip", source.to_string_lossy()),
            ],
        )
    } else {
        run_without_shell(
            "zip",
            &[
                "-r",
                &format!("{}.zip", source.to_string_lossy()),
                &source.to_string_lossy(),
            ],
        )
    }
}

pub fn unzip(source: &Path, destination: &Path) -> StrResult {
    if cfg!(windows) {
        run_without_shell(
            "powershell",
            &[
                "Expand-Archive",
                &source.to_string_lossy(),
                &destination.to_string_lossy(),
            ],
        )
    } else {
        run_without_shell(
            "unzip",
            &[
                &source.to_string_lossy(),
                "-d",
                &destination.to_string_lossy(),
            ],
        )
    }
}

pub fn download(url: &str, destination: &Path) -> StrResult {
    run_without_shell(
        "curl",
        &["-o", &destination.to_string_lossy(), "--url", url],
    )
}

pub fn date_utc_yyyymmdd() -> String {
    let output = if cfg!(windows) {
        Command::new("powershell")
            .arg("(Get-Date).ToUniversalTime().ToString(\"yyyy.MM.dd\")")
            .output()
            .unwrap()
    } else {
        Command::new("date")
            .args(&["-u", "+%Y.%m.%d"])
            .output()
            .unwrap()
    };

    String::from_utf8_lossy(&output.stdout)
        .as_ref()
        .to_owned()
        .replace('\r', "")
        .replace('\n', "")
}
