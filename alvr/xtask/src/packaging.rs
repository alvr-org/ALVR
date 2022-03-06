use crate::{build, command, version};
use alvr_filesystem as afs;
use std::{fs, path::PathBuf};

fn build_windows_installer() {
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

    command::run_without_shell(
        &heat_cmd.to_string_lossy(),
        &[
            "dir",
            r"build\alvr_server_windows",
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
            r"target\wix\harvested.wxs",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            r"-dBuildRoot=build\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            &format!("-dVersion={version}"),
            r"alvr\xtask\wix\main.wxs",
            r"target\wix\harvested.wxs",
            "-o",
            r"target\wix\",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &light_cmd.to_string_lossy(),
        &[
            r"target\wix\main.wixobj",
            r"target\wix\harvested.wixobj",
            "-ext",
            "WixUIExtension",
            "-ext",
            "WixUtilExtension",
            "-o",
            r"target\wix\alvr.msi",
        ],
    )
    .unwrap();

    // Build the bundle including ALVR and vc_redist.
    command::run_without_shell(
        &candle_cmd.to_string_lossy(),
        &[
            "-arch",
            "x64",
            r"-dBuildRoot=build\alvr_server_windows",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            r"alvr\xtask\wix\bundle.wxs",
            "-o",
            r"target\wix\",
        ],
    )
    .unwrap();

    command::run_without_shell(
        &light_cmd.to_string_lossy(),
        &[
            r"target\wix\bundle.wixobj",
            "-ext",
            "WixUtilExtension",
            "-ext",
            "WixBalExtension",
            "-o",
            &format!(r"build\ALVR_Installer_v{version}.exe"),
        ],
    )
    .unwrap();
}

pub fn package_server(root: Option<String>, gpl: bool) {
    build::build_server(true, gpl, root, true, false);

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
            .join("alvr/xtask/licenses_template.hbs")
            .to_string_lossy(),
        licenses_dir.join("dependencies.html").to_string_lossy()
    ))
    .unwrap();
    fs::copy(
        afs::workspace_dir().join("alvr/server/LICENSE-Valve"),
        licenses_dir.join("Valve.txt"),
    )
    .unwrap();
    if gpl {
        fs::copy(
            afs::workspace_dir().join("deps/windows/ffmpeg/LICENSE.txt"),
            licenses_dir.join("FFmpeg.txt"),
        )
        .ok();
    }

    command::zip(&afs::server_build_dir()).unwrap();

    if cfg!(windows) {
        fs::copy(
            afs::target_dir().join("release").join("alvr_server.pdb"),
            afs::build_dir().join("alvr_server.pdb"),
        )
        .unwrap();

        build_windows_installer();
    }
}
