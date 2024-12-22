use crate::{
    build::{self, Profile},
    command,
    dependencies::{self, OpenXRLoadersSelection},
    BuildPlatform,
};
use alvr_filesystem as afs;
use std::{fs, path::Path};
use xshell::{cmd, Shell};

pub enum StreamerReleaseFlavor {
    GitHub,
    SteamStore,
}

pub enum ClientReleaseFlavor {
    GitHub,
    MetaStore,
    PicoStore,
}

pub fn include_licenses(root_path: &Path, gpl: bool) {
    let sh = Shell::new().unwrap();

    // Add licenses
    let licenses_dir = root_path.join("licenses");
    sh.create_dir(&licenses_dir).unwrap();
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

    // Gather licenses with cargo about
    cmd!(sh, "cargo install cargo-about --version 0.6.4")
        .run()
        .unwrap();
    let licenses_template = afs::crate_dir("xtask").join("licenses_template.hbs");
    let licenses_content = cmd!(sh, "cargo about generate {licenses_template}")
        .read()
        .unwrap();
    sh.write_file(licenses_dir.join("dependencies.html"), licenses_content)
        .unwrap();
}

// Note: all packaged release flavors we support are GPL
pub fn package_streamer(
    release_flavor: StreamerReleaseFlavor,
    platform: Option<BuildPlatform>,
    skip_admin_priv: bool,
    enable_nvenc: bool,
    root: Option<String>,
) {
    let sh = Shell::new().unwrap();

    dependencies::prepare_server_deps(platform, skip_admin_priv, enable_nvenc, true);

    build::build_streamer(
        Profile::Distribution,
        true,
        root,
        true,
        false,
        false,
        matches!(release_flavor, StreamerReleaseFlavor::SteamStore),
    );

    include_licenses(&afs::streamer_build_dir(), true);

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

    include_licenses(&afs::launcher_build_dir(), false);

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

pub fn package_client_openxr(flavor: ClientReleaseFlavor, skip_admin_priv: bool) {
    fs::remove_dir_all(afs::deps_dir().join("android_openxr")).ok();

    let openxr_selection = match flavor {
        ClientReleaseFlavor::GitHub => OpenXRLoadersSelection::All,
        ClientReleaseFlavor::MetaStore => OpenXRLoadersSelection::OnlyGeneric,
        ClientReleaseFlavor::PicoStore => OpenXRLoadersSelection::OnlyPico,
    };

    dependencies::build_android_deps(skip_admin_priv, false, openxr_selection);

    if !matches!(flavor, ClientReleaseFlavor::GitHub) {
        replace_client_openxr_manifest(
            r#"package = "alvr.client.stable""#,
            r#"package = "alvr.client""#,
        );
    }

    if matches!(flavor, ClientReleaseFlavor::MetaStore) {
        replace_client_openxr_manifest(r#"value = "all""#, r#"value = "quest2|questpro|quest3""#);
    }

    build::build_android_client(Profile::Distribution);
}

pub fn package_client_lib(link_stdcpp: bool, all_targets: bool) {
    let sh = Shell::new().unwrap();

    build::build_android_client_core_lib(Profile::Distribution, link_stdcpp, all_targets);

    command::zip(&sh, &afs::build_dir().join("alvr_client_core")).unwrap();
}
