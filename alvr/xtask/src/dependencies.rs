use crate::command::{self, run_as_bash as bash, run_as_bash_in as bash_in};
use rand::Rng;
use std::{
    fs,
    io::{self, ErrorKind, Read},
    iter, panic,
    path::{Path, PathBuf},
};

fn download_and_extract(url: &str, target_name: &str) -> PathBuf {
    let random_dir_name = iter::repeat(())
        .map(|_| rand::thread_rng().sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .take(10)
        .collect::<String>();
    let download_dir = std::env::temp_dir().join(random_dir_name);

    // Note: downloaded in-memory instead of on disk
    // todo: display progress
    println!("Downloading {}...", target_name);
    let mut zip_data = vec![];
    ureq::get(url)
        .call()
        .unwrap()
        .into_reader()
        .read_to_end(&mut zip_data)
        .unwrap();

    println!(
        "Extracting {} into {}...",
        target_name,
        download_dir.to_string_lossy()
    );
    zip::read::ZipArchive::new(io::Cursor::new(zip_data))
        .unwrap()
        .extract(&download_dir)
        .unwrap();

    download_dir
}

fn build_rust_android_gradle() {
    const PLUGIN_COMMIT: &str = "6e553c13ef2d9bb40b58a7675b96e0757d1b0443";
    const PLUGIN_VERSION: &str = "0.8.3";

    let download_path = download_and_extract(
        &format!(
            "https://github.com/mozilla/rust-android-gradle/archive/{}.zip",
            PLUGIN_COMMIT
        ),
        "Rust Android Gradle plugin",
    );
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
            panic::panic_any(e);
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
    if cfg!(windows) {
        path.to_string_lossy()
            .replace("C:\\", "/mnt/c/")
            .replace("\\", "/")
    } else {
        path.to_string_lossy().as_ref().to_owned()
    }
}

fn build_ffmpeg(target_os: &str) {
    if target_os == "windows" {
        bash(&format!(
            "sudo apt update && sudo apt remove --auto-remove -y gcc && sudo apt install -y {}",
            "make mingw-w64 mingw-w64-tools binutils-mingw-w64 nasm"
        ))
        .unwrap();
    };

    let mut temp_paths = vec![];

    let ffmpeg_path = download_and_extract(
        "https://github.com/FFmpeg/FFmpeg/archive/n4.3.2.zip",
        "FFmpeg",
    );
    temp_paths.push(ffmpeg_path.clone());
    let ffmpeg_path = ffmpeg_path.join("FFmpeg-n4.3.2");

    // todo: add more video encoders: libkvazaar, OpenH264, libvpx, libx265
    // AV1 encoders are excluded because of lack of hardware accelerated decoding support

    let ffmpeg_platform_flags;
    if target_os == "windows" {
        let x264_path = download_and_extract(
            "https://code.videolan.org/videolan/x264/-/archive/stable/x264-stable.zip",
            "x264",
        );
        temp_paths.push(x264_path.clone());
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
            "{} {} {} {}",
            format!("--extra-cflags=\"-I{}/include\"", x264_wsl2_path),
            format!("--extra-ldflags=\"-L{}/lib\"", x264_wsl2_path),
            "--target-os=mingw32 --cross-prefix=x86_64-w64-mingw32-",
            "--disable-decoders --enable-libx264 --arch=x86_64"
        )
    } else {
        // target_os == "android"
        ffmpeg_platform_flags = "--enable-jni --enable-mediacodec".to_string()
    };

    bash_in(
        &ffmpeg_path,
        &format!(
            "./configure {} {} {} {} {} {} {} {} {}",
            format!(
                "--prefix={}",
                windows_to_wsl2_path(
                    &crate::workspace_dir()
                        .join("deps")
                        .join(target_os)
                        .join("ffmpeg")
                )
            ),
            "--enable-gpl --enable-version3",
            "--disable-static --enable-shared",
            "--disable-programs",
            "--disable-doc",
            "--disable-network",
            format!(
                "--disable-muxers --disable-demuxers --disable-parsers {}",
                "--disable-bsfs --disable-protocols --disable-devices --disable-filters"
            ),
            "--enable-lto",
            ffmpeg_platform_flags
        ),
    )
    .unwrap();
    bash_in(&ffmpeg_path, "make -j$(nproc) && make install").unwrap();

    println!("Cleaning up...");
    for path in temp_paths {
        fs::remove_dir_all(path).ok();
    }
}

pub fn build_deps(target_os: &str) {
    if target_os == "windows" {
        command::run("cargo install wasm-pack").unwrap();
        build_ffmpeg("windows");
    } else if target_os == "android" {
        command::run("rustup target add aarch64-linux-android").unwrap();
        build_rust_android_gradle();
    }
}
