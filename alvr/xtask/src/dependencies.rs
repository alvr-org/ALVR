use crate::command;
use alvr_filesystem as afs;
use std::{fs, io::BufRead};
use xshell::{cmd, Shell};

pub fn choco_install(sh: &Shell, packages: &[&str]) -> Result<(), xshell::Error> {
    cmd!(
        sh,
        "powershell Start-Process choco -ArgumentList \"install {packages...} -y\" -Verb runAs -Wait"
    )
    .run()
}

pub fn prepare_x264_windows() {
    let sh = Shell::new().unwrap();

    const VERSION: &str = "0.164";
    const REVISION: usize = 3086;

    let deps_dir = afs::deps_dir();

    command::download_and_extract_zip(
        &sh,
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

    cmd!(sh, "setx PKG_CONFIG_PATH {deps_dir}").run().unwrap();
}

pub fn prepare_ffmpeg_windows() {
    let sh = Shell::new().unwrap();

    let download_path = afs::deps_dir().join("windows");
    command::download_and_extract_zip(
        &sh,
        &format!(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{}",
            "ffmpeg-n5.1-latest-win64-gpl-shared-5.1.zip"
        ),
        &download_path,
    )
    .unwrap();

    fs::rename(
        download_path.join("ffmpeg-n5.1-latest-win64-gpl-shared-5.1"),
        download_path.join("ffmpeg"),
    )
    .unwrap();
}

pub fn prepare_windows_deps(skip_admin_priv: bool) {
    let sh = Shell::new().unwrap();

    if !skip_admin_priv {
        choco_install(
            &sh,
            &[
                "zip",
                "unzip",
                "llvm",
                "vulkan-sdk",
                "wixtoolset",
                "pkgconfiglite",
            ],
        )
        .unwrap();
    }

    prepare_x264_windows();
    prepare_ffmpeg_windows();
}

pub fn build_ffmpeg_linux(nvenc_flag: bool) {
    let sh = Shell::new().unwrap();

    let download_path = afs::deps_dir().join("linux");
    command::download_and_extract_zip(
        &sh,
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
        "--enable-pic",
        "--enable-rpath",
    ];
    let install_prefix = "--prefix=alvr_build";
    // The reason for 4x$ in LDSOFLAGS var refer to https://stackoverflow.com/a/71429999
    // all varients of --extra-ldsoflags='-Wl,-rpath,$ORIGIN' do not work! don't waste your time trying!
    //
    let config_vars = r#"-Wl,-rpath,'$$$$ORIGIN'"#;

    let _push_guard = sh.push_dir(final_path);
    let _env_vars = sh.push_env("LDSOFLAGS", config_vars);

    if nvenc_flag {
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
        {
            let cuda = pkg_config::Config::new().probe("cuda").unwrap();
            let include_flags = cuda
                .include_paths
                .iter()
                .map(|path| format!("-I{}", path.to_string_lossy()))
                .reduce(|a, b| format!("{a} {b}"))
                .expect("pkg-config cuda entry to have include-paths");
            let link_flags = cuda
                .link_paths
                .iter()
                .map(|path| format!("-L{}", path.to_string_lossy()))
                .reduce(|a, b| format!("{a} {b}"))
                .expect("pkg-config cuda entry to have link-paths");

            let nvenc_flags = &[
                "--enable-encoder=h264_nvenc",
                "--enable-encoder=hevc_nvenc",
                "--enable-nonfree",
                "--enable-cuda-nvcc",
                "--enable-libnpp",
                "--enable-hwaccel=h264_nvenc",
                "--enable-hwaccel=hevc_nvenc",
                "--nvccflags=\"-gencode arch=compute_52,code=sm_52 -O2\"",
                &format!("--extra-cflags=\"{include_flags}\""),
                &format!("--extra-ldflags=\"{link_flags}\""),
            ];

            let flags_combined = flags.join(" ");
            let nvenc_flags_combined = nvenc_flags.join(" ");

            let command =
                format!("./configure {install_prefix} {flags_combined} {nvenc_flags_combined}");

            cmd!(sh, "bash -c {command}").run().unwrap();
        }
    } else {
        cmd!(sh, "./configure {install_prefix} {flags...}")
            .run()
            .unwrap();
    }

    let nproc = cmd!(sh, "nproc").read().unwrap();
    cmd!(sh, "make -j{nproc}").run().unwrap();
    cmd!(sh, "make install").run().unwrap();
}

fn get_oculus_openxr_mobile_loader() {
    let sh = Shell::new().unwrap();

    let temp_sdk_dir = afs::build_dir().join("temp_download");

    // OpenXR SDK v1.0.18. todo: upgrade when new version is available
    command::download_and_extract_zip(
        &sh,
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
        choco_install(&sh, &["unzip", "llvm"]).unwrap();
    }

    cmd!(sh, "rustup target add aarch64-linux-android")
        .run()
        .unwrap();
    cmd!(sh, "cargo install cargo-ndk cbindgen").run().unwrap();

    get_oculus_openxr_mobile_loader();
}

pub fn find_resolved_so_paths(
    bin_or_so: &std::path::Path,
    depends_so: &str,
) -> Vec<std::path::PathBuf> {
    let cmdline = format!(
        "ldd {} | cut -d '>' -f 2 | awk \'{{print $1}}\' | grep {}",
        bin_or_so.display(),
        depends_so
    );
    std::process::Command::new("sh")
        .args(&["-c", &cmdline])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_or(vec![], |mut child| {
            let mut result = std::io::BufReader::new(child.stdout.take().unwrap())
                .lines()
                .filter(|line| line.is_ok())
                .map(|line| std::path::PathBuf::from(line.unwrap()).canonicalize()) // canonicalize resolves symlinks
                .filter(|result| result.is_ok())
                .map(|pp| pp.unwrap())
                .collect::<Vec<_>>();
            result.dedup();
            result
        })
}
