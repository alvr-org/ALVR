use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let base_cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("android");

    let common_cpp_dir = base_cpp_dir.join("ALVR-common");
    let include_cpp_dir = base_cpp_dir.join("app/include");
    let source_cpp_dir = base_cpp_dir.join("app/src/main/cpp");

    let cpp_paths = walkdir::WalkDir::new(&common_cpp_dir)
        .into_iter()
        .chain(walkdir::WalkDir::new(&source_cpp_dir).into_iter())
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    // TODO: do android build only if we're bulding for android
    // (can't use target_os for that, since build.rs isn't being built for android)
    let source_files_paths = cpp_paths.iter().filter(|path| {
        path.extension()
            .filter(|ext| ext.to_string_lossy() == "cpp")
            .is_some()
    });
    let alvr_client_version = format!(r#""{}""#, env!("CARGO_PKG_VERSION"));

    if cfg!(feature = "build-cpp") {
        cc::Build::new()
            .cpp(true)
            .flag("-std=c++17")
            .flag("-fexceptions")
            .flag("-frtti")
            .files(source_files_paths)
            .include(&common_cpp_dir)
            .include(include_cpp_dir)
            .include(&source_cpp_dir)
            .include(source_cpp_dir.join("gl_render_utils"))
            .define("OVR_SDK", None)
            .define("ALVR_CLIENT_VERSION", alvr_client_version.as_ref())
            .cpp_link_stdlib("c++_static")
            .compile("bindings");

        cc::Build::new()
            .cpp(false)
            .files(&[common_cpp_dir.join("reedsolomon").join("rs.c")])
            .compile("bindings_rs_c");
    }

    println!(
        "cargo:rustc-link-search=native={}/app/src/main/jniLibs/arm64-v8a",
        base_cpp_dir.to_string_lossy()
    );

    println!("cargo:rustc-link-lib=log");
    println!("cargo:rustc-link-lib=vrapi");
    println!("cargo:rustc-link-lib=GLESv3");
    println!("cargo:rustc-link-lib=EGL");
    println!("cargo:rustc-link-lib=android");
    println!("cargo:rustc-link-lib=OpenSLES");
    println!("cargo:rustc-link-lib=ovrplatformloader");

    bindgen::builder()
        .clang_arg("-xc++")
        .header(source_cpp_dir.join("bindings.h").to_string_lossy())
        .derive_default(true)
        .generate()
        .expect("bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("bindings.rs");

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}
