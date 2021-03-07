use chrono::Utc;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use semver::{Identifier, Version};
use std::fs;

lazy_static! {
    static ref GRADLE_VERSIONNAME_REGEX: Regex =
        Regex::new(r#"versionName\s+"[\d.]+[0-9A-Za-z-.]*(?:\+[0-9A-Za-z-.]+){0,1}""#).unwrap();
    static ref GRADLE_VERSIONCODE_REGEX: Regex =
        Regex::new(r#"versionCode\s+(?P<code>\d+)"#).unwrap();
}

fn bump_client_gradle_version(new_version: &Version, is_nightly: bool) {
    let old_version = alvr_xtask::version();

    println!(
        "Bumping client version (gradle): {} -> {}",
        old_version, new_version
    );

    let gradle_file_path = crate::workspace_dir()
        .join("alvr/client/android/app")
        .join("build.gradle");
    let file_content = fs::read_to_string(&gradle_file_path).unwrap();

    let file_content = GRADLE_VERSIONNAME_REGEX.replace(&file_content, |_: &Captures| {
        format!(r#"versionName "{}""#, new_version)
    });
    let file_content = if !is_nightly {
        GRADLE_VERSIONCODE_REGEX.replace(&file_content, |caps: &Captures| {
            let code: u32 = (&caps["code"]).parse().unwrap();
            format!("versionCode {}", code + 1)
        })
    } else {
        file_content
    };

    fs::write(gradle_file_path, file_content.as_ref()).unwrap();
}

fn bump_cargo_version(crate_dir_name: &str, new_version: &Version) {
    let manifest_path = crate::workspace_dir()
        .join("alvr")
        .join(crate_dir_name)
        .join("Cargo.toml");

    let mut manifest: toml_edit::Document =
        fs::read_to_string(&manifest_path).unwrap().parse().unwrap();

    manifest["package"]["version"] = toml_edit::value(new_version.to_string());

    fs::write(manifest_path, manifest.to_string_in_original_order()).unwrap();
}

pub fn bump_version(version_arg: Option<&str>, is_nightly: bool) {
    let mut version = if let Some(version_arg) = version_arg {
        Version::parse(version_arg).unwrap()
    } else {
        let mut version = Version::parse(&alvr_xtask::version()).unwrap();
        if !is_nightly {
            version.increment_patch();
        }
        version
    };

    if is_nightly {
        let today = Utc::now().format("%Y%m%d");
        version.build = vec![Identifier::AlphaNumeric(format!("nightly.{}", today))];
    }

    bump_client_gradle_version(&version, is_nightly);
    bump_cargo_version("common", &version);
    bump_cargo_version("server", &version);
    bump_cargo_version("launcher", &version);
    bump_cargo_version("client", &version);

    println!("Git tag:\nv{}", version);
}
