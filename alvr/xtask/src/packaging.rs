use crate::{
    BuildPlatform,
    build::{self, Profile},
    command,
    dependencies::{self, OpenXRLoadersSelection},
};
use alvr_filesystem as afs;
use std::{env, fs, path::Path};
use xshell::{Shell, cmd};

pub enum ReleaseFlavor {
    GitHub,
    MetaStore,
    PicoStore,
}

pub fn generate_licenses(licenses_output: &Path) {
    let sh = Shell::new().unwrap();
    cmd!(sh, "cargo install cargo-about --version 0.8.4 --locked")
        .run()
        .unwrap();
    let licenses_template = afs::crate_dir("xtask").join("licenses_template.hbs");
    cmd!(
        sh,
        "cargo about generate {licenses_template} -o {licenses_output}"
    )
    .run()
    .unwrap();
}

pub fn check_licenses() {
    generate_licenses(&std::env::temp_dir().join("dependencies.html"));
}

pub fn include_base_licenses(root_path: &Path) {
    let sh = Shell::new().unwrap();

    let licenses_dir = root_path.join("licenses");
    sh.create_dir(&licenses_dir).unwrap();
    sh.copy_file(
        afs::workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();

    generate_licenses(&licenses_dir.join("dependencies.html"));
}

pub fn include_licenses(root_path: &Path, gpl: bool) {
    let sh = Shell::new().unwrap();

    // Add licenses
    include_base_licenses(root_path);

    let licenses_dir = root_path.join("licenses");
    sh.copy_file(
        afs::workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();
    sh.copy_file(
        afs::crate_dir("server_openvr").join("LICENSE-Valve"),
        licenses_dir.join("Valve.txt"),
    )
    .unwrap();
    if gpl {
        sh.copy_file(
            afs::deps_dir().join("windows/ffmpeg/LICENSE.txt"),
            licenses_dir.join("FFmpeg.txt"),
        )
        .ok();
    }
    if cfg!(windows) {
        sh.copy_file(
            afs::deps_dir().join("windows/libvpl/alvr_build/share/vpl/licensing/license.txt"),
            licenses_dir.join("libvpl.txt"),
        )
        .unwrap();
    }
}

pub fn package_streamer(
    platform: Option<BuildPlatform>,
    skip_admin_priv: bool,
    enable_nvenc: bool,
    gpl: bool,
    root: Option<String>,
) {
    let sh = Shell::new().unwrap();

    dependencies::prepare_server_deps(platform, skip_admin_priv, enable_nvenc);

    build::build_streamer(Profile::Distribution, gpl, root, true, false, false);

    include_licenses(&afs::streamer_build_dir(), gpl);

    if cfg!(windows) {
        command::zip(&sh, &afs::streamer_build_dir()).unwrap();
    } else {
        command::targz(&sh, &afs::streamer_build_dir()).unwrap();
    }
}

pub fn package_launcher() {
    let sh = Shell::new().unwrap();

    sh.remove_path(afs::launcher_build_dir()).ok();

    build::build_launcher(Profile::Distribution, true);

    include_base_licenses(&afs::launcher_build_dir());

    if cfg!(windows) {
        command::zip(&sh, &afs::launcher_build_dir()).unwrap();

        // todo: installer
    } else {
        command::targz(&sh, &afs::launcher_build_dir()).unwrap();
    }
}

pub fn replace_client_openxr_manifest(from_pattern: &str, to: &str) {
    let manifest_path = afs::crate_dir("client_openxr").join("Cargo.toml");
    let manifest_string = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace(from_pattern, to);

    fs::write(manifest_path, manifest_string).unwrap();
}

/// Replaces the line of the client manifest that starts with `key = `, which must exist.
fn set_client_openxr_manifest_value(key: &str, value: &str) {
    let manifest_path = afs::crate_dir("client_openxr").join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();

    let prefix = format!("{key} = ");
    let mut found = false;
    let manifest = manifest
        .lines()
        .map(|line| {
            if !found && line.starts_with(&prefix) {
                found = true;
                format!("{key} = \"{value}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("
");
    assert!(found, "No `{key}` in {}", manifest_path.display());

    fs::write(manifest_path, manifest + "
").unwrap();
}

pub fn package_client_openxr(flavor: ReleaseFlavor, skip_admin_priv: bool) {
    // Lets a fork ship the client as an app of its own, which installs next to official ALVR rather
    // than conflicting with it. Changes the manifest in place, as the store flavors below do.
    if let Ok(package_id) = env::var("ALVR_ANDROID_PACKAGE_ID") {
        set_client_openxr_manifest_value("package", &package_id);
    }
    if let Ok(label) = env::var("ALVR_ANDROID_LABEL") {
        set_client_openxr_manifest_value("label", &label);
    }

    fs::remove_dir_all(afs::deps_dir().join("android_openxr")).ok();

    let openxr_selection = match flavor {
        ReleaseFlavor::GitHub => OpenXRLoadersSelection::All,
        ReleaseFlavor::MetaStore => OpenXRLoadersSelection::OnlyGeneric,
        ReleaseFlavor::PicoStore => OpenXRLoadersSelection::OnlyPico,
    };

    dependencies::build_android_deps(skip_admin_priv, false, openxr_selection);

    if !matches!(flavor, ReleaseFlavor::GitHub) {
        replace_client_openxr_manifest(
            r#"package = "alvr.client.stable""#,
            r#"package = "alvr.client""#,
        );
    }

    if matches!(flavor, ReleaseFlavor::MetaStore) {
        replace_client_openxr_manifest(r#"value = "all""#, r#"value = "quest2|questpro|quest3""#);
    }

    build::build_android_client(Profile::Distribution);
}

pub fn package_client_lib(link_stdcpp: bool, all_targets: bool) {
    let sh = Shell::new().unwrap();

    build::build_android_client_core_lib(Profile::Distribution, link_stdcpp, all_targets);

    command::zip(&sh, &afs::build_dir().join("alvr_client_core")).unwrap();
}
