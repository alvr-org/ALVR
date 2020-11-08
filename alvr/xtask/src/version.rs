use crate::*;

use lazy_static::lazy_static;
use regex::{Captures, Regex};
use semver::Version;
use std::cmp::Ordering;
use std::fs::File;
use std::str::FromStr;
use toml_edit::Document;

lazy_static! {
    static ref GRADLE_VERSIONNAME_REGEX: Regex = Regex::new(r#"versionName\s+"[\d.]+[0-9A-Za-z-.]*""#).unwrap();
    static ref GRADLE_VERSIONCODE_REGEX: Regex =
        Regex::new(r#"versionCode\s+(?P<code>\d+)"#).unwrap();
}

fn bumped_versions(
    server_version: Option<&str>,
    client_version: Option<&str>,
) -> BResult<(Version, Version)> {
    let old_server_version = alvr_xtask::server_version();
    let old_client_version = alvr_xtask::client_version();

    let server_version = server_version.unwrap_or(&old_server_version);
    let client_version = client_version.unwrap_or(&old_client_version);

    let server_version = Version::parse(server_version)?;
    let client_version = Version::parse(client_version)?;

    if client_version.major != server_version.major {
        return Err("Bumped versions need to have the same major version!"
            .to_owned()
            .into());
    }

    Ok((client_version, server_version))
}

fn bump_client_gradle_version(new_version: &Version) -> BResult {
    let old_client_version = alvr_xtask::client_version();

    println!(
        "Bumping HMD client version: {} -> {}",
        old_client_version, new_version
    );

    let gradle_file_path = workspace_dir()
        .join("alvr/client/android/app")
        .join("build.gradle");
    let mut gradle_file = File::open(&gradle_file_path)?;
    let mut data = String::new();
    gradle_file.read_to_string(&mut data)?;
    drop(gradle_file);

    let data = GRADLE_VERSIONNAME_REGEX.replace(&data, |_: &Captures| {
        format!(r#"versionName "{}""#, new_version)
    });
    let client_version = Version::parse(&old_client_version)?;
    let data = GRADLE_VERSIONCODE_REGEX.replace(&data, |caps: &Captures| {
        if new_version > &client_version {
            let code: u32 = (&caps["code"]).parse().unwrap();
            format!("versionCode {}", code + 1)
        } else {
            format!("versionCode {}", &caps["code"])
        }
    });

    let mut gradle_file = File::create(&gradle_file_path)?;
    gradle_file.write_all(data.as_bytes())?;

    Ok(())
}

fn bump_server_cargo_version(new_version: &Version) -> BResult {
    println!(
        "Bumping server version: {} -> {}",
        alvr_xtask::server_version(),
        new_version
    );
    let manifest_path = workspace_dir()
        .join("alvr/server")
        .join("Cargo.toml");

    let mut manifest = Document::from_str(&fs::read_to_string(&manifest_path)?)?;

    manifest["package"]["version"] = toml_edit::value(new_version.to_string());
    let s = manifest.to_string_in_original_order();
    let new_contents_bytes = s.as_bytes();

    let mut file = File::create(&manifest_path)?;
    file.write_all(new_contents_bytes)?;

    Ok(())
}

pub fn bump_versions(server_arg: Option<String>, client_arg: Option<String>) -> BResult {
    let versions = bumped_versions(server_arg.as_deref(), client_arg.as_deref());
    match versions {
        Ok((client_version, server_version)) => {
            ok_or_exit(bump_client_gradle_version(&client_version));
            ok_or_exit(bump_server_cargo_version(&server_version));

            let tag = match (server_arg, client_arg) {
                (Some(_), Some(_)) | (None, None) => match client_version.cmp(&server_version) {
                    Ordering::Less => format!("v{}-server", server_version),
                    Ordering::Greater => format!("v{}-client", client_version),
                    Ordering::Equal => format!("v{}", client_version),
                },
                (Some(_), None) => format!("v{}-server", server_version),
                (None, Some(_)) => format!("v{}-client", client_version),
            };

            println!("Git tag:\n{}", tag);
            Ok(())
        }
        Err(msg) => Err(format!("Version bump failed: {}", msg).into()),
    }
}
