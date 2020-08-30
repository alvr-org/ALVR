use crate::*;
use alvr_common::data::{ALVR_CLIENT_VERSION, ALVR_SERVER_VERSION};

use semver::{Version, VersionReq};
use gradle_sync::BuildGradleFile;
use toml_edit::Document;
use std::str::FromStr;
use std::fs::File;
use std::cmp::Ordering;

fn bumped_versions(server_version: &Option<String>, client_version: &Option<String>) -> Result<(Version, Version), Box<dyn Error>>{
    let server_version = server_version.as_deref().unwrap_or(ALVR_SERVER_VERSION);
    let client_version = client_version.as_deref().unwrap_or(ALVR_CLIENT_VERSION);

    let server_req: String = format!(">={}", ALVR_SERVER_VERSION).into();
    let server_req = VersionReq::parse(&server_req)?;
    let server_version = Version::parse(server_version)?;

    let client_req: String = format!(">={}", ALVR_CLIENT_VERSION).into();
    let client_req = VersionReq::parse(&client_req)?;
    let client_version = Version::parse(client_version)?;

    if client_version.major != server_version.major {
        return Err("Bumped versions need to have the same major version!".to_owned().into());
    }

    if !server_req.matches(&server_version) {
        return Err(format!("Cannot bump server version: {} -> {}", ALVR_SERVER_VERSION, server_version).into());
    }

    if !client_req.matches(&client_version) {
        return Err(format!("Cannot bump client version: {} -> {}", ALVR_CLIENT_VERSION, client_version).into());
    }

    if client_version == ALVR_CLIENT_VERSION.parse()? && server_version == ALVR_SERVER_VERSION.parse()? {
        Err("Didn't bump any version!".to_owned().into())
    } else {
        Ok((client_version, server_version))
    }
}

fn bump_client_gradle_version(new_version: &Version) -> BResult {
    println!("Bumping HMD client version: {} -> {}", ALVR_CLIENT_VERSION, new_version);

    let gradle_file = BuildGradleFile::new(
        workspace_dir().join("alvr/client_hmd/app").join("build.gradle").to_str().unwrap()
    );

    match gradle_file {
        Ok(mut file) => {
            if let Err(e) = file.sync_version(new_version) {
                return Err(format!("{:#?}", e).into());
            }
            if let Err(e) = file.write() {
                return Err(format!("{:#?}", e).into());
            }
        }
        Err(e) => {
            return Err(format!("{:#?}", e).into());
        }
    }
    Ok(())
}

fn bump_server_cargo_version(new_version: &Version) -> BResult {
    println!("Bumping server version: {} -> {}", ALVR_SERVER_VERSION, new_version);
    let manifest_path = workspace_dir().join("alvr/server_driver").join("Cargo.toml");

    let mut manifest = Document::from_str(
        &fs::read_to_string(&manifest_path)?
    )?;

    manifest["package"]["version"] = toml_edit::value(new_version.to_string());
    let s = manifest.to_string_in_original_order();
    let new_contents_bytes = s.as_bytes();

    let mut file = File::create(&manifest_path)?;
    file.write_all(new_contents_bytes)?;

    Ok(())
}

pub fn bump_versions(server_arg: Option<String>, client_arg: Option<String>) {
    let versions = bumped_versions(&server_arg, &client_arg);
    match versions {
        Ok((client_version, server_version)) => {
            ok_or_exit(bump_client_gradle_version(&client_version));
            ok_or_exit(bump_server_cargo_version(&server_version));

            let tag = if server_arg.is_some() && client_arg.is_some() {
                match client_version.cmp(&server_version) {
                    Ordering::Less => format!("v{}", server_version),
                    Ordering::Greater => format!("v{}", client_version),
                    Ordering::Equal => format!("v{}", client_version)
                }
            } else {
                match client_version.cmp(&server_version) {
                    Ordering::Less => format!("v{}-server", server_version),
                    Ordering::Greater => format!("v{}-client", client_version),
                    Ordering::Equal => format!("v{}", client_version)
                }
            };

            println!("Git tag:\n{}", tag);
        }
        Err(msg) => {
            println!("Version bump failed: {}", msg);
        }
    }
}