use std::{env, path::PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os != "windows" && target_os != "linux" {
        return;
    }

    // let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // let workspace_dir = project_dir.parent().unwrap().parent().unwrap();
    let ffmpeg_src_dir = project_dir.join("src").join("ffmpeg");

    // let ffmpeg_dir = workspace_dir.join("deps").join(&target_os).join("ffmpeg");

    // if target_os == "windows" {
    //     println!(
    //         "cargo:rustc-link-search=native={}",
    //         ffmpeg_dir.join("bin").to_string_lossy()
    //     );
    // }
    // println!("cargo:rustc-link-lib=avcodec");
    // println!("cargo:rustc-link-lib=avdevice");
    // println!("cargo:rustc-link-lib=avfilter");
    // println!("cargo:rustc-link-lib=avformat");
    // println!("cargo:rustc-link-lib=avutil");
    // println!("cargo:rustc-link-lib=postproc");
    // println!("cargo:rustc-link-lib=swresample");
    // println!("cargo:rustc-link-lib=swscale");

    // let mut build = cc::Build::new();
    // let mut build = build
    //     .cpp(false)
    //     .include(&ffmpeg_src_dir)
    //     .file(ffmpeg_src_dir.join("ffmpeg.c"));
    // if target_os != "linux" {
    //     build = build.include(ffmpeg_dir.join("include"));
    // }
    // build.compile("ffmpeg_module");

    // bindgen::builder()
    //     .header(ffmpeg_src_dir.join("ffmpeg.h").to_string_lossy())
    //     .prepend_enum_name(false)
    //     .generate()
    //     .unwrap()
    //     .write_to_file(out_dir.join("ffmpeg_module.rs"))
    //     .unwrap();

    println!(
        "cargo:rerun-if-changed={}",
        ffmpeg_src_dir.to_string_lossy()
    );
}
