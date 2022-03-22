use crate::command;
use alvr_filesystem as afs;
use std::fs;
use xshell::{cmd, Shell};

pub fn choco_install(packages: &[&str]) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    cmd!(
        sh,
        "powershell Start-Process choco -ArgumentList \"install {packages...} -y\" -Verb runAs"
    )
    .run()
}

pub fn prepare_x264_windows() {
    const VERSION: &str = "0.164";
    const REVISION: usize = 3086;

    let deps_dir = afs::deps_dir();

    command::download_and_extract_zip(
        &format!(
            "{}/{VERSION}.r{REVISION}/libx264_{VERSION}.r{REVISION}_msvc16.zip",
            "https://github.com/ShiftMediaProject/x264/releases/download",
        ),
        &afs::deps_dir().join("windows/x264"),
    )
    .unwrap();

    fs::write(
        afs::deps_dir().join("x264.pc"),
        format!(
            r#"
prefix={}
exec_prefix=${{prefix}}/bin/x64
libdir=${{prefix}}/lib/x64
includedir=${{prefix}}/include

Name: x264
Description: x264 library
Version: {VERSION}
Libs: -L${{libdir}} -lx264
Cflags: -I${{includedir}}
"#,
            deps_dir
                .join("windows/x264")
                .to_string_lossy()
                .replace('\\', "/")
        ),
    )
    .unwrap();

    let sh = Shell::new().unwrap();
    cmd!(sh, "setx PKG_CONFIG_PATH {deps_dir}").run().unwrap();
}

pub fn prepare_ffmpeg_windows() {
    let download_path = afs::deps_dir().join("windows");
    command::download_and_extract_zip(
        &format!(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{}",
            "ffmpeg-n5.0-latest-win64-gpl-shared-5.0.zip"
        ),
        &download_path,
    )
    .unwrap();

    fs::rename(
        download_path.join("ffmpeg-n5.0-latest-win64-gpl-shared-5.0"),
        download_path.join("ffmpeg"),
    )
    .unwrap();
}

pub fn prepare_windows_deps(skip_admin_priv: bool) {
    if !skip_admin_priv {
        choco_install(&["llvm", "vulkan-sdk", "wixtoolset", "pkgconfiglite"]).unwrap();
    }

    prepare_x264_windows();
    prepare_ffmpeg_windows();
}

pub fn build_ffmpeg_linux(nvenc_flag: bool) {
    let sh = Shell::new().unwrap();

    let download_path = afs::deps_dir().join("linux");
    command::download_and_extract_zip(
        "https://codeload.github.com/FFmpeg/FFmpeg/zip/n4.4",
        &download_path,
    )
    .unwrap();

    let final_path = download_path.join("ffmpeg");

    fs::rename(download_path.join("FFmpeg-n4.4"), &final_path).unwrap();

    let flags = [
        "--enable-gpl",
        "--enable-version3",
        "--disable-static",
        "--enable-shared",
        "--disable-programs",
        "--disable-doc",
        "--disable-avdevice",
        "--disable-avformat",
        "--disable-swresample",
        "--disable-postproc",
        "--disable-network",
        "--enable-lto",
        "--disable-everything",
        "--enable-encoder=h264_vaapi",
        "--enable-encoder=hevc_vaapi",
        "--enable-encoder=libx264",
        "--enable-encoder=libx264rgb",
        "--enable-encoder=libx265",
        "--enable-hwaccel=h264_vaapi",
        "--enable-hwaccel=hevc_vaapi",
        "--enable-filter=scale",
        "--enable-filter=scale_vaapi",
        "--enable-libx264",
        "--enable-libx265",
        "--enable-vulkan",
        "--enable-libdrm",
    ];

    /*
       Describing Nvidia specific options --nvccflags:
       nvcc from CUDA toolkit version 11.0 or higher does not support compiling for 'compute_30' (default in ffmpeg)
       52 is the minimum required for the current CUDA 11 version (Quadro M6000 , GeForce 900, GTX-970, GTX-980, GTX Titan X)
       https://arnon.dk/matching-sm-architectures-arch-and-gencode-for-various-nvidia-cards/
       Anyway below 50 arch card don't support nvenc encoding hevc https://developer.nvidia.com/nvidia-video-codec-sdk (Supported devices)
       Nvidia docs:
       https://docs.nvidia.com/video-technologies/video-codec-sdk/ffmpeg-with-nvidia-gpu/#commonly-faced-issues-and-tips-to-resolve-them
    */
    #[cfg(target_os = "linux")]
    let nvenc_flags = if nvenc_flag {
        let cuda = pkg_config::Config::new().probe("cuda").unwrap();
        let include_flags = cuda
            .include_paths
            .iter()
            .map(|path| format!("-I{path:?}"))
            .reduce(|a, b| format!("{a}{b}"))
            .expect("pkg-config cuda entry to have include-paths");
        let link_flags = cuda
            .link_paths
            .iter()
            .map(|path| format!("-L{path:?}"))
            .reduce(|a, b| format!("{a}{b}"))
            .expect("pkg-config cuda entry to have link-paths");

        vec![
            "--enable-encoder=h264_nvenc".into(),
            "--enable-encoder=hevc_nvenc".into(),
            "--enable-nonfree".into(),
            "--enable-cuda-nvcc".into(),
            "--enable-libnpp".into(),
            "--nvccflags=\"-gencode arch=compute_52,code=sm_52 -O2\"".into(),
            format!("--extra-cflags=\"{include_flags}\""),
            format!("--extra-ldflags=\"{link_flags}\""),
            "--enable-hwaccel=h264_nvenc".into(),
            "--enable-hwaccel=hevc_nvenc".into(),
        ]
    } else {
        vec![]
    };
    #[cfg(not(target_os = "linux"))]
    let nvenc_flags = Vec::<String>::new();

    let _push_guard = sh.push_dir(final_path);
    cmd!(sh, "./configure {flags...} {nvenc_flags...}")
        .run()
        .unwrap();
    let nproc = cmd!(sh, "nproc").read().unwrap();
    cmd!(sh, "make -j{nproc}").run().unwrap();
}

fn get_oculus_openxr_mobile_loader() {
    let temp_sdk_dir = afs::build_dir().join("temp_download");

    // OpenXR SDK v1.0.18. todo: upgrade when new version is available
    command::download_and_extract_zip(
        "https://securecdn.oculus.com/binaries/download/?id=4421717764533443",
        &temp_sdk_dir,
    )
    .unwrap();

    let destination_dir = afs::deps_dir().join("android/oculus_openxr/arm64-v8a");
    fs::create_dir_all(&destination_dir).unwrap();

    fs::copy(
        temp_sdk_dir.join("OpenXR/Libs/Android/arm64-v8a/Release/libopenxr_loader.so"),
        destination_dir.join("libopenxr_loader.so"),
    )
    .unwrap();

    fs::remove_dir_all(temp_sdk_dir).ok();
}

pub fn build_android_deps(skip_admin_priv: bool) {
    let sh = Shell::new().unwrap();

    if cfg!(windows) && !skip_admin_priv {
        choco_install(&["llvm"]).unwrap();
    }

    cmd!(sh, "rustup target add aarch64-linux-android")
        .run()
        .unwrap();
    cmd!(sh, "cargo install cargo-apk").run().unwrap();

    get_oculus_openxr_mobile_loader();
}
