use alvr_filesystem as afs;
use std::fs;
use std::mem;
use walkdir::{DirEntry, FilterEntry, WalkDir};
use xshell::{cmd, Shell};

fn should_format(entry: &DirEntry) -> bool {
    if entry.file_type().is_dir() {
        match entry.path().file_name().unwrap().to_str().unwrap() {
            "shared" | "include" => false,
            _ => true,
        }
    } else {
        let is_cpp_file = entry
            .path()
            .extension()
            .is_some_and(|ext| matches!(ext.to_str().unwrap(), "c" | "cpp" | "h" | "hpp"));

        let should_skip = entry
            .path()
            .strip_prefix(afs::workspace_dir().join("alvr"))
            .unwrap()
            .to_str()
            .is_some_and(|name| {
                matches!(
                    name,
                    "server/cpp/platform/win32/NvCodecUtils.h"
                        | "server/cpp/platform/win32/NvEncoder.cpp"
                        | "server/cpp/platform/win32/NvEncoder.h"
                        | "server/cpp/platform/win32/NvEncoderD3D11.cpp"
                        | "server/cpp/platform/win32/NvEncoderD3D11.h"
                        | "server/cpp/alvr_server/nvEncodeAPI.h"
                )
            });

        is_cpp_file && !should_skip
    }
}

fn get_files() -> std::iter::Filter<
    FilterEntry<walkdir::IntoIter, impl FnMut(&DirEntry) -> bool>,
    impl FnMut(&Result<DirEntry, walkdir::Error>) -> bool,
> {
    let cpp_dir = afs::crate_dir("server").join("cpp/");

    WalkDir::new(cpp_dir)
        .into_iter()
        .filter_entry(|e| should_format(e))
        .filter(|e| !e.as_ref().unwrap().file_type().is_dir())
}

pub fn format() {
    let sh = Shell::new().unwrap();
    let dir = sh.push_dir(afs::workspace_dir());

    cmd!(sh, "cargo fmt --all").run().unwrap();

    for file in get_files() {
        let path = file.as_ref().unwrap().path();
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

    for file in get_files() {
        let path = file.as_ref().unwrap().path();
        let content = fs::read_to_string(path).unwrap();
        let mut output = cmd!(sh, "clang-format {path}").read().unwrap();

        if content.chars().last().unwrap() != '\n' {
            panic!("file {} missing final newline", path.display());
        }
        output.push('\n');

        if content != output {
            panic!("clang-format check failed for {}", path.display());
        }
    }

    mem::drop(dir);
}
