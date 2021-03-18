use crate::command::{self, run_as_bash as bash, run_as_bash_in as bash_in};
use fs_extra as fsx;
use std::{fs, io::ErrorKind, path::Path};

fn install_rust_android_gradle() {
    const PLUGIN_COMMIT: &str = "6e553c13ef2d9bb40b58a7675b96e0757d1b0443";
    const PLUGIN_VERSION: &str = "0.8.3";

    let rust_android_archive_url = format!(
        "https://github.com/mozilla/rust-android-gradle/archive/{}.zip",
        PLUGIN_COMMIT
    );

    let download_path = cached_path::cached_path_with_options(
        &rust_android_archive_url,
        &cached_path::Options::default().extract(),
    )
    .unwrap();
    let download_path = download_path.join(format!("rust-android-gradle-{}", PLUGIN_COMMIT));

    #[cfg(windows)]
    let gradlew_path = download_path.join("gradlew.bat");
    #[cfg(target_os = "linux")]
    let gradlew_path = download_path.join("gradlew");

    command::run_in(
        &download_path,
        &format!("{} publish", gradlew_path.to_string_lossy()),
    )
    .unwrap();

    let dep_dir = crate::workspace_dir()
        .join("deps")
        .join("rust-android-gradle");
    if let Err(e) = fs::create_dir_all(&dep_dir) {
        if e.kind() != ErrorKind::AlreadyExists {
            panic!(e);
        }
    }

    // Workaround for long path issue on Windows - canonicalize
    let plugin_path = download_path.canonicalize().unwrap();
    let plugin_path = plugin_path
        .join("samples")
        .join("maven-repo")
        .join("org")
        .join("mozilla")
        .join("rust-android-gradle")
        .join("rust-android")
        .join(PLUGIN_VERSION)
        .join(format!("rust-android-{}.jar", PLUGIN_VERSION));
    fs::copy(
        plugin_path,
        dep_dir.join(format!("rust-android-{}.jar", PLUGIN_VERSION)),
    )
    .unwrap();
}

fn windows_to_wsl2_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace("C:\\", "/mnt/c/")
        .replace("\\", "/")
}

fn build_ffmpeg() {
    let deps = if cfg!(windows) {
        "mingw-w64 mingw-w64-tools nasm"
    } else {
        "libx264-dev libvulkan-dev"
    };
    bash(&format!(
        "sudo apt update && sudo apt install -y build-essential {}",
        deps
    ))
    .unwrap();

    // Note: FFmpeg tag n4.3.2
    let ffmpeg_path = cached_path::cached_path_with_options(
        &format!(
            "https://git.ffmpeg.org/gitweb/ffmpeg.git/snapshot/{}.tar.gz",
            "f719f869907764e6412a6af6e178c46e5f915d25"
        ),
        &cached_path::Options::default().extract(),
    )
    .unwrap();
    let ffmpeg_path = ffmpeg_path.join("ffmpeg-f719f86");

    let ffmpeg_platform_flags;
    if cfg!(windows) {
        let x264_path = cached_path::cached_path_with_options(
            "https://code.videolan.org/videolan/x264/-/archive/stable/x264-stable.zip",
            &cached_path::Options::default().extract(),
        )
        .unwrap();
        let x264_path = x264_path.join("x264-stable");

        bash_in(
            &x264_path,
            &format!(
                "./configure --prefix=./build --disable-cli --enable-static {}",
                "--host=x86_64-w64-mingw32 --cross-prefix=x86_64-w64-mingw32-"
            ),
        )
        .unwrap();
        bash_in(&x264_path, "make -j$(nproc) && make install").unwrap();

        let x264_wsl2_path = windows_to_wsl2_path(&x264_path.join("build"));
        ffmpeg_platform_flags = format!(
            "--extra-cflags=\"-I{}/include -fno-use-linker-plugin -fno-lto\" --extra-ldflags=-L{}/lib {}",
            x264_wsl2_path,
            x264_wsl2_path,
            "--target-os=mingw32 --cross-prefix=x86_64-w64-mingw32-"
        )
    } else {
        ffmpeg_platform_flags = "--enable-vulkan --enable-lto".to_string()
    }

    // todo: add more video encoders: libkvazaar, OpenH264, libvpx, libx265
    // AV1 encoders are excluded because of lack of hardware accelerated decoding support
    bash_in(
        &ffmpeg_path,
        &format!(
            "./configure {} {} {} {} {} {} {} {} {} {}",
            "--prefix=./ffmpeg",
            "--enable-gpl --enable-version3",
            "--disable-static --enable-shared",
            "--disable-programs",
            "--disable-doc",
            "--disable-network",
            format!(
                "--disable-decoders --disable-muxers --disable-demuxers --disable-parsers {}",
                "--disable-bsfs --disable-protocols --disable-devices --disable-filters"
            ),
            "--enable-libx264",
            "--arch=x86_64",
            ffmpeg_platform_flags
        ),
    )
    .unwrap();
    bash_in(&ffmpeg_path, "make -j$(nproc) && make install").unwrap();

    fsx::move_items(
        &[ffmpeg_path.join("ffmpeg")],
        crate::workspace_dir().join("deps"),
        &fsx::dir::CopyOptions {
            overwrite: true,
            ..<_>::default()
        },
    )
    .unwrap();
}

pub fn install_deps() {
    command::run("rustup target add aarch64-linux-android").unwrap();
    install_rust_android_gradle();
    build_ffmpeg();
}
