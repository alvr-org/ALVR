use crate::{
    build::{self, Profile},
    command, version,
};
use alvr_filesystem as afs;
use std::path::PathBuf;
use xshell::{cmd, Shell};

fn build_windows_installer() {
    let sh = Shell::new().unwrap();

    let wix_path = PathBuf::from(r"C:\Program Files (x86)\WiX Toolset v3.11\bin");
    let heat_cmd = wix_path.join("heat.exe");
    let candle_cmd = wix_path.join("candle.exe");
    let light_cmd = wix_path.join("light.exe");

    // Clear away build and prerelease version specifiers, MSI can have only dot-separated numbers.
    let mut version = version::version();
    if let Some(idx) = version.find('-') {
        version = version[..idx].to_owned();
    }
    if let Some(idx) = version.find('+') {
        version = version[..idx].to_owned();
    }

    let server_build_dir = afs::server_build_dir();
    let wix_source_dir = afs::crate_dir("xtask").join("wix");
    let wix_target_dir = afs::target_dir().join("wix");
    let main_source = wix_source_dir.join("main.wxs");
    let main_object = wix_target_dir.join("main.wixobj");
    let harvested_source = wix_target_dir.join("harvested.wxs");
    let harvested_object = wix_target_dir.join("harvested.wixobj");
    let alvr_msi = afs::build_dir().join("alvr_server_windows.msi");
    let bundle_source = wix_source_dir.join("bundle.wxs");
    let bundle_object = wix_target_dir.join("bundle.wixobj");
    let installer = afs::build_dir().join(format!("ALVR_Installer_v{version}.exe"));

    cmd!(sh, "{heat_cmd} dir {server_build_dir} -ag -sreg -srd -dr APPLICATIONFOLDER -cg BuildFiles -var var.BuildRoot -o {harvested_source}").run().unwrap();
    cmd!(sh, "{candle_cmd} -arch x64 -dBuildRoot={server_build_dir} -ext WixUtilExtension -dVersion={version} {main_source} {harvested_source} -o {wix_target_dir}\\").run().unwrap();
    cmd!(sh, "{light_cmd} {main_object} {harvested_object} -ext WixUIExtension -ext WixUtilExtension -o {alvr_msi}").run().unwrap();
    cmd!(sh, "{candle_cmd} -arch x64 -dBuildRoot={server_build_dir} -ext WixUtilExtension -ext WixBalExtension {bundle_source} -o {wix_target_dir}\\").run().unwrap();
    cmd!(
        sh,
        "{light_cmd} {bundle_object} -ext WixUtilExtension -ext WixBalExtension -o {installer}"
    )
    .run()
    .unwrap();
}

pub fn package_server(
    root: Option<String>,
    gpl: bool,
    local_ffmpeg: bool,
    appimage: bool,
    zsync: bool,
) {
    let sh = Shell::new().unwrap();

    build::build_server(Profile::Distribution, gpl, root, true, false, local_ffmpeg);

    // Add licenses
    let licenses_dir = afs::server_build_dir().join("licenses");
    sh.create_dir(&licenses_dir).unwrap();
    sh.copy_file(
        afs::workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();
    sh.copy_file(
        afs::crate_dir("server").join("LICENSE-Valve"),
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
    cmd!(sh, "cargo install cargo-about").run().unwrap();
    let licenses_template = afs::crate_dir("xtask").join("licenses_template.hbs");
    let licenses_content = cmd!(sh, "cargo about generate {licenses_template}")
        .read()
        .unwrap();
    sh.write_file(licenses_dir.join("dependencies.html"), licenses_content)
        .unwrap();

    // Finally package everything
    if cfg!(windows) {
        command::zip(&sh, &afs::server_build_dir()).unwrap();

        build_windows_installer();
    } else {
        command::targz(&sh, &afs::server_build_dir()).unwrap();

        if appimage {
            server_appimage(true, gpl, zsync);
        }
    }
}

pub fn server_appimage(release: bool, gpl: bool, update: bool) {
    let sh = Shell::new().unwrap();

    let appdir = &afs::build_dir().join("ALVR.AppDir");
    let bin = &afs::build_dir().join("alvr_server_linux");

    let icon = &afs::workspace_dir().join("resources/alvr.png");
    let desktop = &afs::workspace_dir().join("packaging/freedesktop/alvr.desktop");

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
    // Faster decompression (gzip) or smaller AppImage size (xz)?
    // sh.set_var("APPIMAGE_COMP", "xz"); // Currently uses gzip compression, will take effect when linuxdeploy updates.

    sh.change_dir(&afs::build_dir());
    let mut cmd = cmd!(&sh, "{linuxdeploy} --appdir={appdir} -i{icon} -d{desktop} --deploy-deps-only={appdir}/usr/lib64/alvr/bin/linux64/driver_alvr_server.so --deploy-deps-only={appdir}/usr/lib64/libalvr_vulkan_layer.so --output appimage");

    if gpl {
        for lib_path in sh
            .read_dir(appdir.join("usr/lib64/alvr"))
            .unwrap()
            .into_iter()
            .filter(|path| path.file_name().unwrap().to_string_lossy().contains(".so."))
        {
            let file_name = lib_path.file_name().unwrap().to_string_lossy();
            if file_name.contains("libx264.so") || file_name.contains("libx265.so") {
                sh.remove_path(lib_path).ok();
            } else {
                cmd = cmd.arg(format!("--deploy-deps-only={}", lib_path.to_string_lossy()));
            }
        }
    }

    cmd.run().unwrap();
}

pub fn package_client_lib() {
    let sh = Shell::new().unwrap();

    build::build_client_lib(Profile::Distribution);

    command::zip(&sh, &afs::build_dir().join("alvr_client_core")).unwrap();
}
