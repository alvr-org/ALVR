use std::{env, path::PathBuf};

fn main() {
    let platform_name = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if platform_name == "android" {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = project_dir.parent().unwrap().parent().unwrap();
    let ffmpeg_src_dir = project_dir.join("src").join("ffmpeg");

    let ffmpeg_dir = workspace_dir
        .join("deps")
        .join(platform_name)
        .join("ffmpeg");

    println!(
        "cargo:rustc-link-search=native={}",
        ffmpeg_dir
            .join(if cfg!(windows) { "bin" } else { "lib" })
            .to_string_lossy()
    );
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avdevice");
    println!("cargo:rustc-link-lib=avfilter");
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avutil");
    println!("cargo:rustc-link-lib=postproc");
    println!("cargo:rustc-link-lib=swresample");
    println!("cargo:rustc-link-lib=swscale");

    cc::Build::new()
        .include(ffmpeg_dir.join("include"))
        .include(&ffmpeg_src_dir)
        .file(ffmpeg_src_dir.join("ffmpeg.c"))
        .compile("ffmpeg");

    bindgen::builder()
        .header(ffmpeg_src_dir.join("ffmpeg.h").to_string_lossy())
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("ffmpeg_bindings.rs"))
        .unwrap();

    println!(
        "cargo:rerun-if-changed={}",
        ffmpeg_src_dir.to_string_lossy()
    );
}
