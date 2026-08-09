//! Checks that ffmpeg is available and puts its DLLs next to the executable on Windows.
//!
//! The path itself is supplied by `FFMPEG_DIR` in `.cargo/config.toml`, because `ffmpeg-sys-next`
//! reads that from the process environment and a build script cannot influence a sibling one.

use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");

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
                let destination = target_dir.join(path.file_name().unwrap());

                // Only copy when missing or stale, so incremental builds stay quick.
                let up_to_date = fs::metadata(&destination)
                    .ok()
                    .zip(fs::metadata(&path).ok())
                    .and_then(|(dst, src)| Some((dst.modified().ok()?, src.modified().ok()?)))
                    .is_some_and(|(dst, src)| dst >= src);

                if !up_to_date {
                    fs::copy(&path, &destination).ok();
                }
            }
        }
    }
}

/// The directory the executable is written to.
///
/// Cargo exposes no variable for it, so it is derived from `OUT_DIR`, which sits at
/// `<target>/<profile>/build/<crate>-<hash>/out`.
#[cfg(windows)]
fn target_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").ok()?);

    Some(out_dir.ancestors().nth(3)?.to_owned())
}
