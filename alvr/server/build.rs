use alvr_filesystem as afs;
#[cfg(target_os = "linux")]
use pkg_config;
use std::{env, path::PathBuf};

// this code must be executed BEFORE the actual cpp build when using bundled ffmpeg,
// as it adds definitions and include flags
// but AFTER the build in other cases because linker flags must appear after.
#[cfg(target_os = "linux")]
fn do_ffmpeg_pkg_config(build: &mut cc::Build) {
    let ffmpeg_path = afs::deps_dir().join("linux/ffmpeg");
    let alvr_build = if cfg!(feature = "gpl") {
        "alvr_build"
    } else {
        ""
    };
    let ffmpeg_build_dir = ffmpeg_path.join(alvr_build);

    #[cfg(feature = "gpl")]
    {
        assert!(ffmpeg_path.exists());
        let ffmpeg_pkg_path = ffmpeg_build_dir.join("lib/pkgconfig/");
        assert!(ffmpeg_pkg_path.exists());

        let ffmpeg_pkg_path = ffmpeg_pkg_path.to_string_lossy().to_string();
        env::set_var(
            "PKG_CONFIG_PATH",
            env::var("PKG_CONFIG_PATH").map_or(ffmpeg_pkg_path.clone(), |old| {
                format!("{ffmpeg_pkg_path}:{old}")
            }),
        );
    }

    let pkg = pkg_config::Config::new()
        .cargo_metadata(cfg!(not(feature = "gpl")))
        .to_owned();
    let avutil = pkg.probe("libavutil").unwrap();
    let avfilter = pkg.probe("libavfilter").unwrap();
    let avcodec = pkg.probe("libavcodec").unwrap();
    let swscale = pkg.probe("libswscale").unwrap();

    if cfg!(feature = "gpl") {
        build
            .define("AVCODEC_MAJOR", avcodec.version.split(".").next().unwrap())
            .define("AVUTIL_MAJOR", avutil.version.split(".").next().unwrap())
            .define(
                "AVFILTER_MAJOR",
                avfilter.version.split(".").next().unwrap(),
            )
            .define("SWSCALE_MAJOR", swscale.version.split(".").next().unwrap());

        assert!(ffmpeg_build_dir.join("include").exists());
        build.include(ffmpeg_build_dir.join("include"));

        // activate dlopen for libav libraries
        build
            .define("LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN", None)
            .define("LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN", None)
            .define("LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN", None)
            .define("LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN", None);

        println!("cargo:rustc-link-lib=dl");
    }
}

#[cfg(all(windows, feature = "gpl"))]
fn do_ffmpeg_windows_config(build: &mut cc::Build) {
    let ffmpeg_dir = afs::deps_dir().join("windows/ffmpeg");

    build.include(format!("{}/include", ffmpeg_dir.to_string_lossy()));

    println!(
        "cargo:rustc-link-search=native={}/lib",
        ffmpeg_dir.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avutil");
    println!("cargo:rustc-link-lib=avfilter");
    println!("cargo:rustc-link-lib=swscale");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cpp");

    #[cfg(windows)]
    let platform = "cpp/platform/win32";
    #[cfg(target_os = "linux")]
    let platform = "cpp/platform/linux";
    #[cfg(target_os = "macos")]
    let platform = "cpp/platform/macos";

    let common_iter = walkdir::WalkDir::new("cpp")
        .into_iter()
        .filter_entry(|entry| entry.file_name() != "tools" && entry.file_name() != "platform");

    let platform_iter = walkdir::WalkDir::new(platform).into_iter();

    let cpp_paths = common_iter
        .chain(platform_iter)
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    let source_files_paths = cpp_paths.iter().filter(|path| {
        path.extension()
            .filter(|ext| {
                let ext_str = ext.to_string_lossy();
                ext_str == "c" || ext_str == "cpp"
            })
            .is_some()
    });

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .files(source_files_paths)
        .flag_if_supported("-isystemcpp/openvr/headers") // silences many warnings from openvr headers
        .flag_if_supported("-std=c++17")
        .include("cpp/openvr/headers")
        .include("cpp");

    #[cfg(windows)]
    build
        .debug(false) // This is because we cannot link to msvcrtd (see below)
        .flag("/std:c++17")
        .flag("/permissive-")
        .define("NOMINMAX", None)
        .define("_WINSOCKAPI_", None)
        .define("_MBCS", None)
        .define("_MT", None);

    #[cfg(target_os = "macos")]
    build.define("__APPLE__", None);

    // #[cfg(debug_assertions)]
    // build.define("ALVR_DEBUG_LOG", None);

    #[cfg(all(target_os = "linux", feature = "gpl"))]
    do_ffmpeg_pkg_config(&mut build);

    #[cfg(all(windows, feature = "gpl"))]
    {
        do_ffmpeg_windows_config(&mut build);
        build.define("ALVR_GPL", None);
    }

    build.compile("bindings");

    #[cfg(all(target_os = "linux", not(feature = "gpl")))]
    do_ffmpeg_pkg_config(&mut build);

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/alvr_server/bindings.h")
        .derive_default(true)
        .generate()
        .expect("bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("bindings.rs");

    println!(
        "cargo:rustc-link-search=native={}",
        cpp_dir.join("openvr/lib").to_string_lossy()
    );
    println!("cargo:rustc-link-lib=openvr_api");

    #[cfg(target_os = "linux")]
    {
        pkg_config::Config::new().probe("vulkan").unwrap();

        // fail build if there are undefined symbols in final library
        println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    }

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}
