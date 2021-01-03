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

pub fn version() -> String {
    let manifest_path = packages_dir().join("common").join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.to_string_lossy());

    let manifest: toml_edit::Document = fs::read_to_string(manifest_path).unwrap().parse().unwrap();

    manifest["package"]["version"].as_str().unwrap().into()
}
