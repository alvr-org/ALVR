extern crate meson;
use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_path = out_dir.to_str().unwrap();

    println!("cargo:rustc-link-lib=alvr");
    println!("cargo:rustc-link-search=native={}", build_path);
    meson::build("cpp", build_path);

    bindgen::builder()
        .clang_arg("-xc++")
        .header("cpp/alvr_server/bindings.h")
        .derive_default(true)
        .generate()
        .expect("bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("bindings.rs");

    if cfg!(not(windows)) {
        //FIXME: we should not have to declare libs both here and in meson
        println!("cargo:rustc-link-lib=stdc++");
    }

    let cpp_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cpp");
    println!(
        "cargo:rustc-link-search=native={}/openvr/lib",
        cpp_dir.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=openvr_api");
}
