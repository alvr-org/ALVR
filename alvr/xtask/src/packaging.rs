use crate::{
    build::{self, Profile},
    command,
    dependencies::{self, OpenXRLoadersSelection},
    version,
};
use alvr_filesystem as afs;
use std::{
    env::consts::OS,
    fs,
    path::{Path, PathBuf},
};
use xshell::{cmd, Shell};

pub enum ReleaseFlavor {
    GitHub,
    MetaStore,
    PicoStore,
}

fn build_windows_installer() {
    let sh = Shell::new().unwrap();

    let wix_path = PathBuf::from(r"C:\Program Files (x86)\WiX Toolset v3.11\bin");
    let heat_cmd = wix_path.join("heat.exe");
    let candle_cmd = wix_path.join("candle.exe");
    let light_cmd = wix_path.join("light.exe");

    // Clear away build and prerelease version specifiers, MSI can have only dot-separated numbers.
    let mut version = version::version();
    if let Some(idx) = version.find('-') {
        version.drain(idx..);
    }
    if let Some(idx) = version.find('+') {
        version.drain(idx..);
    }

    let streamer_build_dir = afs::streamer_build_dir();
    let wix_source_dir = afs::crate_dir("xtask").join("wix");
    let wix_target_dir = afs::target_dir().join("wix");
    let main_source = wix_source_dir.join("main.wxs");
    let main_object = wix_target_dir.join("main.wixobj");
    let harvested_source = wix_target_dir.join("harvested.wxs");
    let harvested_object = wix_target_dir.join("harvested.wixobj");
    let alvr_msi = afs::build_dir().join("alvr_streamer_windows.msi");
    let bundle_source = wix_source_dir.join("bundle.wxs");
    let bundle_object = wix_target_dir.join("bundle.wixobj");
    let installer = afs::build_dir().join(format!("ALVR_Installer_v{version}.exe"));

    cmd!(sh, "{heat_cmd} dir {streamer_build_dir} -ag -sreg -srd -dr APPLICATIONFOLDER -cg BuildFiles -var var.BuildRoot -o {harvested_source}").run().unwrap();
    cmd!(sh, "{candle_cmd} -arch x64 -dBuildRoot={streamer_build_dir} -ext WixUtilExtension -dVersion={version} {main_source} {harvested_source} -o {wix_target_dir}\\").run().unwrap();
    cmd!(sh, "{light_cmd} {main_object} {harvested_object} -ext WixUIExtension -ext WixUtilExtension -o {alvr_msi}").run().unwrap();
    cmd!(sh, "{candle_cmd} -arch x64 -dBuildRoot={streamer_build_dir} -ext WixUtilExtension -ext WixBalExtension {bundle_source} -o {wix_target_dir}\\").run().unwrap();
    cmd!(
        sh,
        "{light_cmd} {bundle_object} -ext WixUtilExtension -ext WixBalExtension -o {installer}"
    )
    .run()
    .unwrap();
}

fn package_streamer_appimage(release: bool, update: bool) {
    let sh = Shell::new().unwrap();

    let appdir = &afs::build_dir().join("ALVR.AppDir");
    let bin = &afs::build_dir().join("alvr_streamer_linux");

    let icon = &afs::workspace_dir().join("resources/alvr.png");
    let desktop = &afs::workspace_dir().join("alvr/xtask/resources/alvr.desktop");

    let linuxdeploy = afs::build_dir().join("linuxdeploy-x86_64.AppImage");

    if !sh.path_exists(&linuxdeploy) {
        command::download(&sh, "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage", &linuxdeploy).ok();
    }
    cmd!(&sh, "chmod a+x {linuxdeploy}").run().ok();

    if sh.path_exists(appdir) {
        sh.remove_path(appdir).ok();
    }

    cmd!(&sh, "{linuxdeploy} --appdir={appdir}").run().ok();

    sh.cmd("sh")
        .arg("-c")
        .arg(format!(
            "cp -r {}/* {}/usr",
            bin.to_string_lossy(),
            appdir.to_string_lossy()
        ))
        .run()
        .ok();

    sh.set_var("ARCH", "x86_64");
    sh.set_var("OUTPUT", "ALVR-x86_64.AppImage");

    if release {
        let version = version::version();
        sh.set_var("VERSION", &version);

        if update {
            let repo = if version.contains("nightly") {
                "ALVR-nightly"
            } else {
                "ALVR"
            };
            sh.set_var(
                "UPDATE_INFORMATION",
                format!("gh-releases-zsync|alvr-org|{repo}|latest|ALVR-x86_64.AppImage.zsync"),
            );
        }
    }

    sh.set_var("VERBOSE", "1");
    sh.set_var("NO_APPSTREAM", "1");
    // sh.set_var("APPIMAGE_COMP", "xz");

    sh.change_dir(afs::build_dir());

    cmd!(&sh, "{linuxdeploy} --appdir={appdir} -i{icon} -d{desktop} --deploy-deps-only={appdir}/usr/lib64/alvr/bin/linux64/driver_alvr_server.so --deploy-deps-only={appdir}/usr/lib64/libalvr_vulkan_layer.so --output appimage").run().unwrap();
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

pub fn package_streamer(gpl: bool, root: Option<String>, appimage: bool, zsync: bool) {
    let sh = Shell::new().unwrap();

    build::build_streamer(
        Profile::Distribution,
        !appimage,
        gpl,
        root,
        true,
        false,
        false,
    );

    include_licenses(&afs::streamer_build_dir(), gpl);

    if OS == "windows" {
        command::zip(&sh, &afs::streamer_build_dir()).unwrap();

        // todo: remove installer
        // note: wix package is broken, find alternative
        //build_windows_installer();
    } else {
        command::targz(&sh, &afs::streamer_build_dir()).unwrap();

        if appimage {
            package_streamer_appimage(true, zsync);
        }
    }
}

pub fn package_launcher(appimage: bool) {
    let sh = Shell::new().unwrap();

    build::build_launcher(Profile::Distribution, !appimage, true);

    include_licenses(&afs::launcher_build_dir(), false);

    if OS == "windows" {
        command::zip(&sh, &afs::launcher_build_dir()).unwrap();

        // todo: installer
    } else {
        command::targz(&sh, &afs::launcher_build_dir()).unwrap();

        // todo: appimage
    }
}

pub fn replace_client_openxr_manifest(from_pattern: &str, to: &str) {
    let manifest_path = afs::crate_dir("client_openxr").join("Cargo.toml");
    let manifest_string = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace(from_pattern, to);

    fs::write(manifest_path, manifest_string).unwrap();
}

pub fn package_client_openxr(flavor: ReleaseFlavor) {
    crate::clean();

    let openxr_selection = match flavor {
        ReleaseFlavor::GitHub => OpenXRLoadersSelection::All,
        ReleaseFlavor::MetaStore => OpenXRLoadersSelection::OnlyGeneric,
        ReleaseFlavor::PicoStore => OpenXRLoadersSelection::OnlyPico,
    };

    dependencies::build_android_deps(false, false, openxr_selection);

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
