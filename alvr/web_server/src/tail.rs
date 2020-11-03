use alvr_common::*;
use std::{path::Path, process::Stdio};
use tokio::{io::AsyncBufReadExt, io::BufReader, process::Command, stream::Stream};

#[cfg(not(windows))]
fn tail_command(fname: &str) -> Command {
    let mut command = Command::new("tail");
    command.args(&["-F", fname]); // todo: -F for debug purposes, change to -f
    command
}
#[cfg(windows)]
fn tail_command(file_path: &Path) -> Command {
    let mut command = Command::new("PowerShell.exe");
    command
        .args(&["Get-Content", &file_path.to_string_lossy(), "-Wait"])
        .creation_flags(commands::CREATE_NO_WINDOW);
    command
}

pub fn tail_stream(file_path: &Path) -> StrResult<impl Stream<Item = std::io::Result<String>>> {
    let process = trace_err!(tail_command(file_path).stdout(Stdio::piped()).spawn())?;
    Ok(BufReader::new(trace_none!(process.stdout)?).lines())
}
