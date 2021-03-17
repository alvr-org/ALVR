use crate::BResult;
use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

pub fn run_as_shell_in(workdir: &Path, shell: &str, shell_flag: &str, cmd: &str) -> BResult {
    println!("\n{}", cmd);

    let output = Command::new(shell)
        .args(&[shell_flag, cmd])
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

pub fn run_in(workdir: &Path, cmd: &str) -> BResult {
    let shell = if cfg!(windows) { "cmd" } else { "bash" };
    let shell_flag = if cfg!(windows) { "/C" } else { "-c" };

    run_as_shell_in(workdir, shell, shell_flag, cmd)
}

pub fn run(cmd: &str) -> BResult {
    run_in(&env::current_dir().unwrap(), cmd)
}

// Bash can be invoked on Windows if WSL is installed
pub fn run_as_bash_in(workdir: &Path, cmd: &str) -> BResult {
    run_as_shell_in(workdir, "bash", "-c", cmd)
}

pub fn run_as_bash(cmd: &str) -> BResult {
    run_as_bash_in(&env::current_dir().unwrap(), cmd)
}

pub fn run_without_shell(cmd: &str, args: &[&str]) -> BResult {
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
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}
