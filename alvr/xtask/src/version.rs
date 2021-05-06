use crate::command;
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
    let gradle_file_path = crate::workspace_dir()
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

    println!("Git tag:\nv{}", version);
}
