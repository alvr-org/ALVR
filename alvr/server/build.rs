use std::{env, path::PathBuf};

fn get_ffmpeg_path() -> PathBuf {
    let ffmpeg_path = alvr_filesystem::deps_dir()
        .join(if cfg!(target_os = "linux") {
            "linux"
        } else {
            "windows"
        })
        .join("ffmpeg");

    if cfg!(target_os = "linux") {
        ffmpeg_path.join("alvr_build")
    } else {
        ffmpeg_path
    }
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

    let use_ffmpeg = cfg!(feature = "gpl") || cfg!(target_os = "linux");

    if use_ffmpeg {
        let ffmpeg_path = get_ffmpeg_path();

        assert!(ffmpeg_path.join("include").exists());
        build.include(ffmpeg_path.join("include"));
    }

    #[cfg(feature = "gpl")]
    build.define("ALVR_GPL", None);

    build.compile("bindings");

    if use_ffmpeg {
        let ffmpeg_path = get_ffmpeg_path();
        let ffmpeg_lib_path = ffmpeg_path.join("lib");

        assert!(ffmpeg_lib_path.exists());

        println!(
            "cargo:rustc-link-search=native={}",
            ffmpeg_lib_path.to_string_lossy()
        );

        #[cfg(target_os = "linux")]
        {
            let ffmpeg_pkg_path = ffmpeg_lib_path.join("pkgconfig");
            assert!(ffmpeg_pkg_path.exists());

            let ffmpeg_pkg_path = ffmpeg_pkg_path.to_string_lossy().to_string();
            env::set_var(
                "PKG_CONFIG_PATH",
                env::var("PKG_CONFIG_PATH").map_or(ffmpeg_pkg_path.clone(), |old| {
                    format!("{ffmpeg_pkg_path}:{old}")
                }),
            );

            let pkg = pkg_config::Config::new().statik(true).to_owned();

            for lib in ["libavutil", "libavfilter", "libavcodec"] {
                pkg.probe(lib).unwrap();
            }
        }
        #[cfg(windows)]
        for lib in ["avutil", "avfilter", "avcodec", "swscale"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/alvr_server/bindings.h")
        .derive_default(true)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

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
