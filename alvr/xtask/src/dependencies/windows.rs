use crate::command;
use alvr_filesystem as afs;
use std::fs;
use xshell::{Shell, cmd};

pub fn choco_install(sh: &Shell, packages: &[&str]) -> Result<(), xshell::Error> {
    cmd!(
        sh,
        "powershell Start-Process choco -ArgumentList \"install {packages...} -y\" -Verb runAs -Wait"
    )
    .run()
}

pub fn prepare_deps(skip_admin_priv: bool) {
    let sh = Shell::new().unwrap();

    let deps_path = deps_path();
    sh.remove_path(&deps_path).ok();
    sh.create_dir(&deps_path).unwrap();

    if !skip_admin_priv {
        choco_install(
            &sh,
            &["zip", "unzip", "llvm", "vulkan-sdk", "pkgconfiglite"],
        )
        .unwrap();
    }

    prepare_prebuilt_x264_windows();
    prepare_prebuilt_ffmpeg_windows();
}

fn deps_path() -> std::path::PathBuf {
    afs::deps_dir().join("windows")
}

fn x264_path() -> std::path::PathBuf {
    deps_path().join("x264")
}

fn ffmpeg_path() -> std::path::PathBuf {
    deps_path().join("ffmpeg")
}

fn prepare_prebuilt_x264_windows() {
    const VERSION: &str = "0.164";
    const REVISION: usize = 3086;

    let sh = Shell::new().unwrap();

    let x264_src_path = x264_path();

    command::download_and_extract_zip(
        &format!(
            "{}/{VERSION}.r{REVISION}/libx264_{VERSION}.r{REVISION}_msvc16.zip",
            "https://github.com/ShiftMediaProject/x264/releases/download",
        ),
        &x264_src_path,
    )
    .unwrap();

    fs::write(
        afs::deps_dir().join("x264.pc"),
        format!(
            r"
prefix={}
exec_prefix=${{prefix}}/bin/x64
libdir=${{prefix}}/lib/x64
includedir=${{prefix}}/include

Name: x264
Description: x264 library
Version: {VERSION}
Libs: -L${{libdir}} -lx264
Cflags: -I${{includedir}}
",
            x264_src_path.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();

    cmd!(sh, "setx PKG_CONFIG_PATH {x264_src_path}")
        .run()
        .unwrap();
}

fn prepare_prebuilt_ffmpeg_windows() {
    let deps_path = deps_path();
    let ffmpeg_path = ffmpeg_path();

    command::download_and_extract_zip(
        &format!(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{}",
            "ffmpeg-n7.1-latest-win64-gpl-shared-7.1.zip"
        ),
        &deps_path,
    )
    .unwrap();

    fs::rename(
        deps_path.join("ffmpeg-n7.1-latest-win64-gpl-shared-7.1"),
        ffmpeg_path,
    )
    .unwrap();
}
