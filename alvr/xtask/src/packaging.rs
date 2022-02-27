use crate::{build_client, build_server, command, version};
use alvr_filesystem as afs;
use std::{env, fs, path::PathBuf};

fn build_windows_installer(wix_path: &str) {
    let wix_path = PathBuf::from(wix_path).join("bin");
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

    command::run_without_shell(
        &heat_cmd.to_string_lossy(),
        &[
            "dir",
            "build\\alvr_server_windows",
            "-ag",
            "-sreg",
            "-srd",
            "-dr",
            "APPLICATIONFOLDER",
            "-cg",
            "BuildFiles",
            "-var",
            "var.BuildRoot",
            "-o",
            "target\\wix\\harvested.wxs",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            &format!("-dVersion={version}"),
            "alvr\\xtask\\wix\\main.wxs",
            "target\\wix\\harvested.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &light_cmd.to_string_lossy(),
        &[
            "target\\wix\\main.wixobj",
            "target\\wix\\harvested.wixobj",
            "-ext",
            "WixUIExtension",
            "-ext",
            "WixUtilExtension",
            "-o",
            "target\\wix\\alvr.msi",
        ],
    )
    .unwrap();

    // Build the bundle including ALVR and vc_redist.
    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            "-dBuildRoot=build\\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "alvr\\xtask\\wix\\bundle.wxs",
            "-o",
            "target\\wix\\",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &light_cmd.to_string_lossy(),
        &[
            "target\\wix\\bundle.wixobj",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "-o",
            &format!("build\\ALVR_Installer_v{version}.exe"),
        ],
    )
    .unwrap();
}

pub fn publish_server(is_nightly: bool, root: Option<String>, reproducible: bool, gpl: bool) {
    let bundle_ffmpeg = cfg!(target_os = "linux");
    build_server(
        true,
        false,
        false,
        bundle_ffmpeg,
        false,
        gpl,
        root,
        reproducible,
    );

    // Add licenses
    let licenses_dir = afs::server_build_dir().join("licenses");
    fs::create_dir_all(&licenses_dir).unwrap();
    fs::copy(
        afs::workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();
    command::run("cargo install cargo-about").unwrap();
    command::run(&format!(
        "cargo about generate {} > {}",
        afs::workspace_dir()
            .join("alvr")
            .join("xtask")
            .join("licenses_template.hbs")
            .to_string_lossy(),
        licenses_dir.join("dependencies.html").to_string_lossy()
    ))
    .unwrap();
    fs::copy(
        afs::workspace_dir()
            .join("alvr")
            .join("server")
            .join("LICENSE-Valve"),
        licenses_dir.join("Valve.txt"),
    )
    .unwrap();
    fs::copy(
        afs::workspace_dir()
            .join("alvr")
            .join("server")
            .join("LICENSE-FFmpeg"),
        licenses_dir.join("FFmpeg.txt"),
    )
    .unwrap();

    command::zip(&afs::server_build_dir()).unwrap();

    if cfg!(windows) {
        if is_nightly {
            fs::copy(
                afs::target_dir().join("release").join("alvr_server.pdb"),
                afs::build_dir().join("alvr_server.pdb"),
            )
            .unwrap();
        }

        if let Some(wix_evar) = env::vars().find(|v| v.0 == "WIX") {
            println!("Found WiX, will build installer.");

            build_windows_installer(&wix_evar.1);
        } else {
            println!("No WiX toolset installation found, skipping installer.");
        }
    }
}

pub fn publish_client(is_nightly: bool) {
    build_client(!is_nightly, is_nightly, false);
    build_client(!is_nightly, is_nightly, true);
}
