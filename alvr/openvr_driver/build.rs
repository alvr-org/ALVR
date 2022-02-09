use cbindgen::Config;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let alvr_streamer_header_dir = Path::new("../../build");
    let alvr_streamer_header_path = alvr_streamer_header_dir.join("alvr_streamer.h");

    cbindgen::Builder::new()
        .with_config(Config::from_file("../server/cbindgen.toml").unwrap())
        .with_crate("../server")
        .generate()
        .unwrap()
        .write_to_file(&alvr_streamer_header_path);

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .files([
            "cpp/paths.cpp",
            "cpp/tracked_device.cpp",
            "cpp/hmd.cpp",
            "cpp/controller.cpp",
            "cpp/generic_tracker.cpp",
            "cpp/chaperone.cpp",
            "cpp/driver.cpp",
        ])
        .include("cpp")
        .include(alvr_streamer_header_dir)
        .compile("bindings");

    bindgen::builder()
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .header("cpp/bindings.h")
        .derive_default(true)
        .enable_cxx_namespaces()
        .prepend_enum_name(false)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

    let artifacts_dir = out_dir.join("../../..");

    #[cfg(windows)]
    fs::copy(
        artifacts_dir.join("alvr_server.dll.lib"),
        artifacts_dir.join("alvr_server.lib"),
    )
    .ok();

    println!(
        "cargo:rustc-link-search={}",
        artifacts_dir.to_string_lossy()
    );

    // Note: compilation problems when using static lib
    // todo: rename to alvr_streamer
    println!("cargo:rustc-link-lib=dylib=alvr_server");

    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("resources/lib")
            .to_string_lossy()
    );
    println!("cargo:rustc-link-lib=openvr_api");

    println!("cargo:rerun-if-changed=cpp");
}
