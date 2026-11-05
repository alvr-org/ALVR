use crate::command;
use alvr_filesystem as afs;
use std::fs;
use xshell::Shell;

pub fn split_string(source: &str, start_pattern: &str, end: char) -> (String, String, String) {
    let start_idx = source.find(start_pattern).unwrap() + start_pattern.len();
    let end_idx = start_idx + source[start_idx..].find(end).unwrap();

    (
        source[..start_idx].to_owned(),
        source[start_idx..end_idx].to_owned(),
        source[end_idx..].to_owned(),
    )
}

pub fn version() -> String {
    let manifest_path = afs::workspace_dir().join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.to_string_lossy());

    let manifest = fs::read_to_string(manifest_path).unwrap();
    let (_, version, _) = split_string(&manifest, "version = \"", '\"');

    version
}

fn bump_cargo_version(new_version: &str) {
    let manifest_path = afs::workspace_dir().join("Cargo.toml");

    let manifest = fs::read_to_string(&manifest_path).unwrap();

    let (file_start, _, file_end) = split_string(&manifest, "version = \"", '\"');
    let manifest = format!("{file_start}{new_version}{file_end}");

    fs::write(manifest_path, manifest).unwrap();
}

pub fn bump_version(maybe_version: Option<String>, is_nightly: bool) {
    let sh = Shell::new().unwrap();

    let mut version = maybe_version.unwrap_or_else(version);

    if is_nightly {
        version = format!(
            "{version}+nightly.{}",
            command::date_utc_yyyymmdd(&sh).unwrap()
        );
    }

    bump_cargo_version(&version);

    println!("Git tag:\nv{version}");
}
