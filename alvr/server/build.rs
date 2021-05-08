use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cpp");

    #[cfg(windows)]
    let platform = "cpp/platform/win32";
    #[cfg(target_os = "linux")]
    let platform = "cpp/platform/linux";

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
        .flag_if_supported("/std:c++17")
        .include("cpp/openvr/headers")
        .include("cpp");

    #[cfg(windows)]
    build
        .debug(false) // This is because we cannot link to msvcrtd (see below)
        .define("NOMINMAX", None)
        .define("_WINSOCKAPI_", None)
        .define("_MBCS", None)
        .define("_MT", None);

    // #[cfg(debug_assertions)]
    // build.define("ALVR_DEBUG_LOG", None);

    build.compile("bindings");

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/alvr_server/bindings.h")
        .derive_default(true)
        .generate()
        .expect("bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("bindings.rs");

    println!(
        "cargo:rustc-link-search=native={}/openvr/lib",
        cpp_dir.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=openvr_api");

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=vulkan");
        println!("cargo:rustc-link-lib=avutil");
        println!("cargo:rustc-link-lib=avcodec");
        println!("cargo:rustc-link-lib=avfilter");
        println!("cargo:rustc-link-lib=swscale");

        // fail build if there are undefined symbols in final library
        println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    }

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}
