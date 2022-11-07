use std::{env, path::PathBuf};

fn main() {
    let platform_name = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let cpp_paths = walkdir::WalkDir::new("cpp")
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    let source_files_paths = if platform_name == "android" {
        cpp_paths
            .iter()
            .filter_map(|path| {
                path.extension()
                    .filter(|ext| ext.to_string_lossy() == "cpp")
                    .is_some()
                    .then(|| path.clone())
            })
            .collect()
    } else {
        vec![
            PathBuf::new().join("cpp/fec.cpp"),
            PathBuf::new().join("cpp/nal.cpp"),
        ]
    };

    let mut builder = &mut cc::Build::new();
    builder = builder
        .cpp(true)
        .files(source_files_paths)
        .include("cpp")
        .include("cpp/gl_render_utils");
    if platform_name == "windows" {
        builder = builder.flag("/std:c++17")
    } else {
        builder = builder
            .flag("-std=c++17")
            .flag("-fexceptions")
            .flag("-frtti")
    }
    if platform_name == "android" {
        builder = builder.cpp_link_stdlib("c++_static");
    }
    builder.compile("bindings");

    cc::Build::new()
        .cpp(false)
        .files(&["cpp/reedsolomon/rs.c"])
        .compile("bindings_rs_c");

    if platform_name == "android" {
        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=EGL");
        println!("cargo:rustc-link-lib=GLESv3");
        println!("cargo:rustc-link-lib=android");
    }

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/bindings.h")
        .derive_default(true)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

    for path in cpp_paths {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
}
