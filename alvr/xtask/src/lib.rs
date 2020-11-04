use regex::Regex;
use std::{fs, path::Path, path::PathBuf};

fn packages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .into()
}

fn get_version(dir_name: &str) -> String {
    let cargo_path = packages_dir().join(dir_name).join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", cargo_path.to_string_lossy());

    let cargo_data: toml_edit::Document = fs::read_to_string(cargo_path).unwrap().parse().unwrap();

    cargo_data["package"]["version"].as_str().unwrap().into()
}

pub fn server_version() -> String {
    get_version("server")
}

pub fn client_version() -> String {
    let re = Regex::new(r#"versionName\s+"(?P<name>[\d.]+[0-9A-Za-z-.]*)""#).unwrap();
    re.captures(
        &fs::read_to_string(packages_dir().join("client/android/app").join("build.gradle")).unwrap(),
    )
    .unwrap()["name"]
        .into()
}
