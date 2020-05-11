use alvr_common::*;
use std::process::Stdio;
use tokio::{
    io::AsyncBufReadExt,
    io::{AsyncRead, BufReader},
    process::Command,
    stream::Stream,
};

const TRACE_CONTEXT: &str = "Tail command";

#[cfg(not(windows))]
fn tail_byte_reader(fname: &str) -> StrResult<impl AsyncRead> {
    Ok(trace_none!(
        trace_err!(Command::new("tail")
            .args(&["--follow", fname])
            .stdout(Stdio::piped())
            .spawn())?
        .stdout
    ))
}
#[cfg(windows)]
fn tail_byte_reader(fname: &str) -> StrResult<impl AsyncRead> {
    Ok(trace_none!(
        trace_err!(Command::new("PowerShell.exe")
            .args(&["Get-Content", fname, "-Wait"])
            .stdout(Stdio::piped())
            .spawn())?
        .stdout
    )?)
}

pub fn tail_stream(fname: &str) -> StrResult<impl Stream<Item = std::io::Result<String>>> {
    Ok(BufReader::new(tail_byte_reader(fname)?).lines())
}
