use std::{env, path::PathBuf};

fn main() {
    let platform_name = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if platform_name == "android" {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        let cpp_paths = walkdir::WalkDir::new("cpp")
            .into_iter()
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.into_path())
            .collect::<Vec<_>>();

        if cfg!(feature = "use-cpp") {
            let source_files_paths = cpp_paths
                .iter()
                .filter(|&path| {
                    path.extension()
                        .filter(|ext| ext.to_string_lossy() == "cpp")
                        .is_some()
                })
                .cloned()
                .collect::<Vec<_>>();

            cc::Build::new()
                .cpp(true)
                .files(source_files_paths)
                .include("cpp")
                .include("cpp/gl_render_utils")
                .flag("-std=c++17")
                .flag("-fexceptions")
                .flag("-frtti")
                .cpp_link_stdlib("c++_static")
                .compile("bindings");

            bindgen::builder()
                .clang_arg("-xc++")
                .header("cpp/bindings.h")
                .derive_default(true)
                .generate()
                .unwrap()
                .write_to_file(out_dir.join("bindings.rs"))
                .unwrap();
        }

        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=EGL");
        println!("cargo:rustc-link-lib=GLESv3");
        println!("cargo:rustc-link-lib=android");

        #[cfg(feature = "link-stdcpp-shared")]
        println!("cargo:rustc-link-lib=c++_shared");

        for path in cpp_paths {
            println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        }
    }
}
