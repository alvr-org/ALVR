#[cfg(target_os = "linux")]
fn main() {
    use std::{env, path::PathBuf};

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let server_cpp_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../server/cpp");

    let vulkan = pkg_config::Config::new().probe("vulkan").unwrap();
    let libunwind = pkg_config::Config::new().probe("libunwind").unwrap();

    let cpp_paths = walkdir::WalkDir::new(".")
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    let source_files_paths = cpp_paths.iter().filter(|path| {
        path.extension()
            .filter(|ext| ext.to_string_lossy() == "cpp")
            .is_some()
    });

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .files(source_files_paths)
        .flag("-std=c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .define("VK_USE_PLATFORM_XLIB_XRANDR_EXT", None)
        .include(cpp_dir)
        .include(server_cpp_dir)
        .includes(vulkan.include_paths)
        .includes(libunwind.include_paths);

    build.compile("VkLayer_ALVR");

    bindgen::builder()
        .clang_arg("-xc++")
        .header("layer/layer.h")
        .derive_default(true)
        .generate()
        .expect("layer bindings")
        .write_to_file(out_dir.join("layer_bindings.rs"))
        .expect("layer_bindings.rs");

    for lib in libunwind.libs {
        println!("cargo:rustc-link-lib={lib}");
    }

    // fail build if there are undefined symbols in final library
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
