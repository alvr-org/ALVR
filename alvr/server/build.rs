use std::{env, path::PathBuf};

#[cfg(windows)]
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cpp");

    let cpp_paths = walkdir::WalkDir::new("cpp")
        .into_iter()
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
        .debug(false) // This is because we cannot link to msvcrtd (see below)
        .cpp(true)
        .files(source_files_paths)
        .include("cpp/alvr_server")
        .include("cpp/shared")
        .include("cpp/openvr/headers")
        .include("cpp/alvr_server/include")
        .include("cpp/libswresample/include")
        .include("cpp/ALVR-common")
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

    if cfg!(windows) {
        println!(
            "cargo:rustc-link-search=native={}/openvr/lib",
            cpp_dir.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=openvr_api");
    }

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}

#[cfg(not(windows))]
fn main() {}
