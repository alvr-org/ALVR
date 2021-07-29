use crate::command;
use crate::workspace_dir;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn packages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

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
    let manifest_path = packages_dir().join("common").join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.to_string_lossy());

    let manifest = fs::read_to_string(manifest_path).unwrap();
    let (_, version, _) = split_string(&manifest, "version = \"", '\"');

    version
}

fn bump_client_gradle_version(new_version: &str, is_nightly: bool) {
    let gradle_file_path = workspace_dir()
        .join("alvr/client/android/app")
        .join("build.gradle");
    let file_content = fs::read_to_string(&gradle_file_path).unwrap();

    // Replace versionName
    let (file_start, _, file_end) = split_string(&file_content, "versionName \"", '\"');
    let file_content = format!("{}{}{}", file_start, new_version, file_end);

    let file_content = if !is_nightly {
        // Replace versionCode
        let (file_start, old_version_code_string, file_end) =
            split_string(&file_content, "versionCode ", '\n');
        format!(
            "{}{}{}",
            file_start,
            old_version_code_string.parse::<usize>().unwrap() + 1,
            file_end
        )
    } else {
        file_content
    };

    fs::write(gradle_file_path, file_content).unwrap();
}

fn bump_cargo_version(crate_dir_name: &str, new_version: &str) {
    let manifest_path = packages_dir().join(crate_dir_name).join("Cargo.toml");

    let manifest = fs::read_to_string(&manifest_path).unwrap();

    let (file_start, _, file_end) = split_string(&manifest, "version = \"", '\"');
    let manifest = format!("{}{}{}", file_start, new_version, file_end);

    fs::write(manifest_path, manifest).unwrap();
}

fn bump_rpm_spec_version(new_version: &str) {
    let spec_path = workspace_dir().join("packaging/rpm/alvr.spec");
    let spec = fs::read_to_string(&spec_path).unwrap();
    let version_start;
    let version_end;

    // If there's a '-', split the version around it
    if new_version.contains("-") {
        let (_, tmp_start, mut tmp_end) = split_string(new_version, "", '-');
        tmp_end.remove(0);
        version_start = tmp_start.to_string();
        version_end = format!("0.0.1{}", tmp_end.to_string());
    } else {
        version_start = new_version.to_string();
        version_end = "1.0.0".to_string();
    }

    // Replace Version
    let (file_start, _, file_end) = split_string(&spec, "Version: ", '\n');
    let spec = format!("{}{}{}", file_start, version_start, file_end);

    // Reset Release to 1.0.0
    let (file_start, _, file_end) = split_string(&spec, "Release: ", '\n');
    let spec = format!("{}{}{}", file_start, version_end, file_end);

    // Replace Source in github URL
    let (file_start, _, file_end) = split_string(&spec, "refs/tags/v", 't');
    let spec = format!("{}{}.{}", file_start, new_version, file_end);

    fs::write(spec_path, spec).unwrap();
}

pub fn bump_version(maybe_version: Option<String>, is_nightly: bool) {
    let mut version = maybe_version.unwrap_or_else(version);

    if is_nightly {
        version = format!("{}+nightly.{}", version, command::date_utc_yyyymmdd());
    }

    bump_client_gradle_version(&version, is_nightly);
    bump_cargo_version("common", &version);
    bump_cargo_version("server", &version);
    bump_cargo_version("launcher", &version);
    bump_cargo_version("client", &version);
    bump_rpm_spec_version(&version);

    println!("Git tag:\nv{}", version);
}
