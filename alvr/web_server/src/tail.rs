use alvr_common::*;
use std::process::Stdio;
use tokio::{io::AsyncBufReadExt, io::BufReader, process::Command, stream::Stream};

const TRACE_CONTEXT: &str = "Tail command";

#[cfg(not(windows))]
fn tail_command(fname: &str) -> Command {
    let mut command = Command::new("tail");
    command.args(&["-F", fname]); // todo: -F for debug purposes, change to -f
    command
}
#[cfg(windows)]
fn tail_command(fname: &str) -> Command {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("PowerShell.exe");
    command
        .args(&["Get-Content", fname, "-Wait"])
        .creation_flags(CREATE_NO_WINDOW);
    command
}

pub fn tail_stream(fname: &str) -> StrResult<impl Stream<Item = std::io::Result<String>>> {
    let process = trace_err!(tail_command(fname).stdout(Stdio::piped()).spawn())?;
    Ok(BufReader::new(trace_none!(process.stdout)?).lines())
}
