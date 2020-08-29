use crate::*;
use alvr_common::data::{ALVR_CLIENT_VERSION, ALVR_SERVER_VERSION};

use semver::{Version, VersionReq};
use gradle_sync::BuildGradleFile;
use toml_edit::Document;
use std::str::FromStr;
use std::fs::File;

fn bumped_versions(server_version: Option<String>, client_version: Option<String>) -> Result<(String, String), String>{
    let server_version = server_version.as_deref().unwrap_or(ALVR_SERVER_VERSION);
    let client_version = client_version.as_deref().unwrap_or(ALVR_CLIENT_VERSION);

    let server_req: String = format!(">={}", ALVR_SERVER_VERSION).into();
    let server_req = VersionReq::parse(&server_req).unwrap();
    let server_version = Version::parse(server_version).unwrap();

    let client_req: String = format!(">={}", ALVR_CLIENT_VERSION).into();
    let client_req = VersionReq::parse(&client_req).unwrap();
    let client_version = Version::parse(client_version).unwrap();

    if client_version.major != server_version.major {
        return Err("Bumped versions need to have the same major version!".to_owned());
    }

    let server_version = if server_req.matches(&server_version) {
        format!("{}", server_version)
    } else {
        return Err(format!("Cannot bump server version: {} -> {}", ALVR_SERVER_VERSION, server_version));
    };

    let client_version = if client_req.matches(&client_version) {
        format!("{}", client_version)
    } else {
        return Err(format!("Cannot bump client version: {} -> {}", ALVR_CLIENT_VERSION, client_version));
    };

    if client_version == ALVR_CLIENT_VERSION && server_version == ALVR_SERVER_VERSION {
        Err("Didn't bump any version!".to_owned())
    } else {
        Ok((client_version, server_version))
    }
}

fn bump_client_gradle_version(new_version: String) {
    println!("Bumping HMD client version: {} -> {}", ALVR_CLIENT_VERSION, new_version);

    let new_version = Version::parse(&new_version).unwrap();

    let mut gradle_file = BuildGradleFile::new(
        workspace_dir().join("alvr/client_hmd/app").join("build.gradle").to_str().unwrap()
    ).unwrap();
    gradle_file.sync_version(&new_version).unwrap();
    gradle_file.write().unwrap();
}

fn bump_server_cargo_version(new_version: String) {
    println!("Bumping server version: {} -> {}", ALVR_SERVER_VERSION, new_version);
    let manifest_path = workspace_dir().join("alvr/server_driver").join("Cargo.toml");

    let mut manifest = Document::from_str(
        &fs::read_to_string(&manifest_path).unwrap()
    ).unwrap();

    manifest["package"]["version"] = toml_edit::value(new_version);
    let s = manifest.to_string_in_original_order();
    let new_contents_bytes = s.as_bytes();

    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(new_contents_bytes).unwrap();
}

pub fn bump_versions(server_version: Option<String>, client_version: Option<String>) {
    let versions = bumped_versions(server_version, client_version);
    match versions {
        Ok((client_version, server_version)) => {
            bump_client_gradle_version(client_version);
            bump_server_cargo_version(server_version);
        }
        Err(msg) => {
            println!("Version bump failed: {}", msg);
        }
    }
}