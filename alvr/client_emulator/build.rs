//! Puts the bundled models next to the executable, checks that ffmpeg is available and, on
//! Windows, puts its DLLs next to the executable too.
//!
//! The ffmpeg path itself is supplied by `FFMPEG_DIR` in `.cargo/config.toml`, because
//! `ffmpeg-sys-next` reads that from the process environment and a build script cannot influence a
//! sibling one.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// The repository directory whose files are copied into `<target>/<profile>/models/`, which is
/// where the settings files' relative model paths resolve.
const MODELS_DIR: &str = "models";

/// The ALVR icon recoloured orange to tell it apart from the dashboard. `main.rs` includes the same file for the window.
const ICON_PATH: &str = "resources/emulator.ico";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={MODELS_DIR}");
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    println!("cargo:rerun-if-changed={ICON_PATH}");

    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon(ICON_PATH);
        resource.compile().unwrap();
    }

    if let Some(target_dir) = target_dir() {
        copy_models(&target_dir.join(MODELS_DIR));
    }

    let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") else {
        // Nothing to check: on Linux pkg-config locates ffmpeg without a directory hint.
        return;
    };
    let ffmpeg_dir = PathBuf::from(ffmpeg_dir);

    if !ffmpeg_dir.join("include").exists() {
        if cfg!(windows) {
            // Failing here with an explanation beats a screen of missing-symbol errors later.
            panic!(
                "ffmpeg headers not found at {}.\nRun `cargo xtask prepare-deps --platform windows` \
                 first, or set FFMPEG_DIR to an ffmpeg install containing include/ and lib/.",
                ffmpeg_dir.display()
            );
        }

        return;
    }

    // ffmpeg is linked dynamically, so the DLLs have to sit beside the executable to run it from
    // the target directory. Copying them here keeps `cargo run` working with no extra step.
    #[cfg(windows)]
    {
        let Some(target_dir) = target_dir() else {
            return;
        };

        let Ok(entries) = fs::read_dir(ffmpeg_dir.join("bin")) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "dll") {
                copy_if_stale(&path, &target_dir.join(path.file_name().unwrap()));
            }
        }
    }
}

/// Copies the bundled models beside the executable, leaving any other file there alone, so models
/// a user converted or downloaded into the same directory survive a rebuild.
fn copy_models(destination: &Path) {
    let Ok(entries) = fs::read_dir(MODELS_DIR) else {
        return;
    };

    fs::create_dir_all(destination).ok();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            copy_if_stale(&path, &destination.join(path.file_name().unwrap()));
        }
    }
}

/// Only copies when the destination is missing or older, so incremental builds stay quick.
fn copy_if_stale(source: &Path, destination: &Path) {
    let up_to_date = fs::metadata(destination)
        .ok()
        .zip(fs::metadata(source).ok())
        .and_then(|(dst, src)| Some((dst.modified().ok()?, src.modified().ok()?)))
        .is_some_and(|(dst, src)| dst >= src);

    if !up_to_date {
        fs::copy(source, destination).ok();
    }
}

/// The directory the executable is written to.
///
/// Cargo exposes no variable for it, so it is derived from `OUT_DIR`, which sits at
/// `<target>/<profile>/build/<crate>-<hash>/out`.
fn target_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").ok()?);

    Some(out_dir.ancestors().nth(3)?.to_owned())
}
