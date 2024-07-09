use alvr_filesystem as afs;
use std::fs;
use std::mem;
use std::path::PathBuf;
use walkdir::WalkDir;
use xshell::{cmd, Shell};

fn files_to_format_paths() -> Vec<PathBuf> {
    let cpp_dir = afs::crate_dir("server_openvr").join("cpp");

    WalkDir::new(cpp_dir)
        .into_iter()
        .filter_entry(|entry| {
            let included = entry.path().is_dir()
                || entry
                    .path()
                    .extension()
                    .is_some_and(|ext| matches!(ext.to_str().unwrap(), "c" | "cpp" | "h" | "hpp"));

            let excluded = matches!(
                entry.file_name().to_str().unwrap(),
                "shared"
                    | "include"
                    | "NvCodecUtils.h"
                    | "NvEncoder.cpp"
                    | "NvEncoder.h"
                    | "NvEncoderD3D11.cpp"
                    | "NvEncoderD3D11.h"
                    | "nvEncodeAPI.h"
            );

            included && !excluded
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().is_file().then(|| entry.path().to_owned())
        })
        .collect()
}

pub fn format() {
    let sh = Shell::new().unwrap();
    let dir = sh.push_dir(afs::workspace_dir());

    cmd!(sh, "cargo fmt --all").run().unwrap();

    for path in files_to_format_paths() {
        cmd!(sh, "clang-format -i {path}").run().unwrap();
    }

    mem::drop(dir);
}

pub fn check_format() {
    let sh = Shell::new().unwrap();
    let dir = sh.push_dir(afs::workspace_dir());

    cmd!(sh, "cargo fmt --all -- --check")
        .run()
        .expect("cargo fmt check failed");

    for path in files_to_format_paths() {
        let content = fs::read_to_string(&path).unwrap();
        let mut output = cmd!(sh, "clang-format {path}").read().unwrap();

        if !content.ends_with('\n') {
            panic!("file {} missing final newline", path.display());
        }
        output.push('\n');

        if content != output {
            panic!("clang-format check failed for {}", path.display());
        }
    }

    mem::drop(dir);
}
