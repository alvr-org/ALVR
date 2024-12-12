use serde::Deserialize;
use serde_json::{self as json, Deserializer, Value};
use xshell::{cmd, Shell};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Level {
    Error,
    Warning,
    Note,
    Help,
    FailureNote,
}

#[derive(Debug, Deserialize)]
struct Span {
    file_name: String,
    line_start: u64,
    line_end: u64,
    column_start: u64,
    column_end: u64,
    is_primary: bool,
}

// https://doc.rust-lang.org/rustc/json.html
#[derive(Debug, Deserialize)]
struct CompilerMessage {
    #[serde(rename = "$message_type")]
    message_type: String,
    level: Level,
    spans: Vec<Span>,
    rendered: String,
}

pub fn clippy_ci() {
    let sh = Shell::new().unwrap();
    let out = cmd!(sh, "cargo clippy --message-format=json --color=always")
        .ignore_status()
        .output()
        .unwrap();

    std::print!("{}", String::from_utf8_lossy(&out.stderr));

    let stream = Deserializer::from_slice(&out.stdout).into_iter::<Value>();

    // https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages
    for message in stream.filter_map(|msg| {
        let msg = msg.unwrap();

        if msg.get("reason")? == "compiler-message" {
            msg.get("message").map(|x| x.to_owned())
        } else {
            None
        }
    }) {
        let msg: CompilerMessage = json::from_value(message).unwrap();

        if msg.message_type != "diagnostic" {
            continue;
        }

        let level = match msg.level {
            Level::Error => Some("error"),
            Level::Warning => Some("warning"),
            Level::Note | Level::Help => Some("notice"),
            _ => None,
        };

        if let Some(level) = level {
            let span = msg
                .spans
                .iter()
                .find(|&sp| sp.is_primary)
                .or(msg.spans.first())
                .unwrap();

            // may break when xtask gets cross-compiled, but that should not happen esp in ci
            let file_name = if cfg!(windows) {
                &span.file_name.replace('\\', "/")
            } else {
                &span.file_name
            };

            // https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions
            println!(
                "::{level} file={},line={},endLine={},col={},endColumn={}::{}",
                file_name,
                span.line_start,
                span.line_end,
                span.column_start,
                span.column_end,
                msg.rendered
                    .replace('%', "%25")
                    .replace('\r', "%0D")
                    .replace('\n', "%0A"),
            );
        }
    }

    if !out.status.success() {
        panic!("ci clippy didn't exit with 0 code, propagating failure");
    }
}
