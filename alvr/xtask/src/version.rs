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

fn bump_rpm_spec_version(new_version: &str, is_nightly: bool) {
    let spec_path = afs::workspace_dir().join("packaging/rpm/alvr.spec");
    let spec = fs::read_to_string(&spec_path).unwrap();

    // If there's a '-', split the version around it
    let (version_start, version_end) = {
        if new_version.contains('-') {
            let (_, tmp_start, mut tmp_end) = split_string(new_version, "", '-');
            tmp_end.remove(0);
            (tmp_start, format!("0.0.1{tmp_end}"))
        } else {
            (new_version.to_owned(), "1.0.0".to_owned())
        }
    };

    // Replace Version
    let (file_start, _, file_end) = split_string(&spec, "Version: ", '\n');
    let spec = format!("{file_start}{version_start}{file_end}");

    // Reset Release to 1.0.0
    let (file_start, _, file_end) = split_string(&spec, "Release: ", '\n');
    let spec = format!("{file_start}{version_end}{file_end}");

    // Replace Source in github URL
    let spec = {
        if is_nightly {
            spec
        } else {
            // Grab version (ex: https://github.com/alvr-org/ALVR/archive/refs/tags/v16.0.0-rc1.tar.gz)
            let (file_start, _, file_end) = split_string(&spec, "refs/tags/v", 't');
            format!("{file_start}{new_version}.{file_end}")
        }
    };

    fs::write(spec_path, spec).unwrap();
}

fn bump_deb_control_version(new_version: &str) {
    let control_path = afs::workspace_dir().join("packaging/deb/control");
    let control = fs::read_to_string(&control_path).unwrap();

    // Replace Version
    let (file_start, _, file_end) = split_string(&control, "\nVersion: ", '\n');
    let control = format!("{file_start}{new_version}{file_end}");

    fs::write(control_path, control).unwrap();
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
    bump_rpm_spec_version(&version, is_nightly);
    bump_deb_control_version(&version);

    println!("Git tag:\nv{version}");
}
