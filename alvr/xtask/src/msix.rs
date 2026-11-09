//! `cargo xtask package-msix`: one MSIX package holding the streamer and the client emulator.
//!
//! The package folder looks like this:
//!
//! ```text
//! AppxManifest.xml      resources/msix/AppxManifest.xml, with the version filled in
//! resources.pri         generated, so Windows can pick the right logo size
//! Assets/               resources/msix/Assets
//! ALVR/                 the streamer, as `build-streamer` lays it out, plus licenses
//! Emulator/             the emulator, its ffmpeg DLLs and its models
//! ```
//!
//! The dashboard copies `ALVR/` to a per-user folder when it runs; see alvr/dashboard/src/msix.rs.
//!
//! Needs the Windows SDK (makeappx, makepri, signtool), and `cargo xtask prepare-deps --platform
//! windows` to have been run. The package is signed when `MSIX_CERT_PFX` names a .pfx file, with
//! the password in `MSIX_CERT_PASSWORD`; otherwise it is left unsigned and cannot be installed.

use crate::{
    build::{self, Profile},
    packaging,
};
use alvr_filesystem as afs;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use xshell::{Shell, cmd};

/// Files in the streamer build that are not shipped: settings and logs from a local run, and
/// debug symbols.
const SKIPPED_EXTENSIONS: &[&str] = &["pdb"];
const SKIPPED_FILES: &[&str] = &["session.json", "session_log.txt", "crash_log.txt"];

pub fn package_msix(version: Option<String>, gpl: bool, reproducible: bool) {
    let sh = Shell::new().unwrap();

    // The streamer build replaces the files in the build folder, which fails half way through, and
    // leaves that folder emptied, while SteamVR has the driver there loaded.
    let tasks = cmd!(sh, "tasklist /FI \"IMAGENAME eq vrserver.exe\" /NH")
        .read()
        .unwrap_or_default();
    if tasks.to_lowercase().contains("vrserver.exe") {
        panic!("SteamVR is running. Close it first: the build replaces the driver it has loaded.");
    }

    let version = msix_version(&version.unwrap_or_else(crate::version::version));
    println!("MSIX package version {version}");

    let stage = afs::build_dir().join("alvr_msix");
    sh.remove_path(&stage).ok();
    sh.create_dir(&stage).unwrap();

    // Streamer. Keeps the session.json of a local build; copy_dir leaves it out of the package.
    build::build_streamer(Profile::Distribution, gpl, None, reproducible, false, true);
    copy_dir(&afs::streamer_build_dir(), &stage.join("ALVR"));
    packaging::include_licenses(&stage.join("ALVR"), gpl);

    // Emulator. Its build script puts the models next to the executable.
    {
        let locked = reproducible.then_some("--locked");
        cmd!(
            sh,
            "cargo build -p alvr_client_emulator --profile distribution {locked...}"
        )
        .run()
        .unwrap();

        let artifacts = afs::target_dir().join("distribution");
        let emulator = stage.join("Emulator");
        sh.create_dir(&emulator).unwrap();
        sh.copy_file(
            artifacts.join(afs::exec_fname("alvr_client_emulator")),
            &emulator,
        )
        .unwrap();
        copy_dir(&artifacts.join("models"), &emulator.join("models"));

        for dll in sh
            .read_dir(afs::deps_dir().join("windows/ffmpeg/bin"))
            .unwrap()
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "dll"))
        {
            sh.copy_file(dll, &emulator).unwrap();
        }
    }

    // Manifest and logos
    let resources = afs::crate_dir("xtask").join("resources/msix");
    copy_dir(&resources.join("Assets"), &stage.join("Assets"));
    let manifest = fs::read_to_string(resources.join("AppxManifest.xml"))
        .unwrap()
        .replace("{VERSION}", &version);
    fs::write(stage.join("AppxManifest.xml"), manifest).unwrap();

    let sdk = windows_sdk_bin_dir();
    let makepri = sdk.join("makepri.exe");
    let makeappx = sdk.join("makeappx.exe");
    let signtool = sdk.join("signtool.exe");

    // The resource index lets Windows choose among the logo sizes in Assets.
    {
        let config = afs::build_dir().join("msix_priconfig.xml");
        let index = stage.join("resources.pri");
        let manifest = stage.join("AppxManifest.xml");
        cmd!(
            sh,
            "{makepri} createconfig /cf {config} /dq en-US /pv 10.0.0 /o"
        )
        .run()
        .unwrap();
        cmd!(
            sh,
            "{makepri} new /pr {stage} /cf {config} /mn {manifest} /of {index} /o"
        )
        .run()
        .unwrap();
    }

    let package = afs::build_dir().join("alvr_windows.msix");
    cmd!(sh, "{makeappx} pack /d {stage} /p {package} /o")
        .run()
        .unwrap();

    match env::var("MSIX_CERT_PFX") {
        Ok(pfx) if !pfx.is_empty() => {
            let password = env::var("MSIX_CERT_PASSWORD").unwrap_or_default();
            cmd!(
                sh,
                "{signtool} sign /fd SHA256 /f {pfx} /p {password} {package}"
            )
            .quiet()
            .run()
            .unwrap();
            println!("Signed {}", package.display());
        }
        _ => println!(
            "MSIX_CERT_PFX is not set, so {} is unsigned and cannot be installed",
            package.display()
        ),
    }
}

/// Seconds from the Unix epoch to 2026-01-01 00:00 UTC, where build numbers start counting.
const BUILD_NUMBER_START: u64 = 1_767_225_600;

/// MSIX versions are four numbers, each at most 65535. The first three are ALVR's version without
/// its pre-release part (`21.0.0-dev12` gives `21.0.0`). The fourth is the build number: the hours
/// since 2026-01-01 UTC, so a later build is always a newer version, wherever it was built, and
/// installs as an update to an earlier one. It runs out in June 2033; by then ALVR's version has
/// most likely moved on, and counting can start again from a later date.
///
/// `MSIX_BUILD_NUMBER` replaces the build number, and `--version` with four numbers replaces the
/// whole version.
fn msix_version(version: &str) -> String {
    let (release, _) = version.split_once('-').unwrap_or((version, ""));
    let mut parts = release
        .split('.')
        .map(|part| {
            part.parse::<u16>().unwrap_or_else(|_| {
                panic!("{part} in version {version} is not a number up to 65535")
            })
        })
        .collect::<Vec<_>>();

    if parts.len() == 4 {
        // Given in full with --version.
    } else {
        parts.resize(3, 0);
        parts.push(build_number());
    }

    parts
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn build_number() -> u16 {
    if let Ok(number) = env::var("MSIX_BUILD_NUMBER") {
        return number
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("MSIX_BUILD_NUMBER {number} is not a number up to 65535"));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hours = now.saturating_sub(BUILD_NUMBER_START) / 3600;

    u16::try_from(hours).unwrap_or_else(|_| {
        panic!(
            "{hours} hours since 2026-01-01 no longer fit in an MSIX version. Move \
             BUILD_NUMBER_START to a later date together with a bump of ALVR's version, or Windows \
             will see new builds as older than the installed one."
        )
    })
}

/// Copies a folder, leaving out what `SKIPPED_FILES` and `SKIPPED_EXTENSIONS` name.
fn copy_dir(source: &Path, destination: &Path) {
    for entry in WalkDir::new(source).into_iter().map(Result::unwrap) {
        let relative = entry.path().strip_prefix(source).unwrap();
        let target = destination.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap();
            continue;
        }

        let name = entry.file_name().to_string_lossy();
        let skipped = SKIPPED_FILES.contains(&name.as_ref())
            || entry
                .path()
                .extension()
                .is_some_and(|ext| SKIPPED_EXTENSIONS.iter().any(|skipped| ext == *skipped));

        if !skipped {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The newest Windows SDK's x64 tools folder, which has makeappx, makepri and signtool.
fn windows_sdk_bin_dir() -> PathBuf {
    let kits = PathBuf::from(
        env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()),
    )
    .join(r"Windows Kits\10\bin");

    let mut versions = fs::read_dir(&kits)
        .unwrap_or_else(|e| panic!("Windows SDK not found at {}: {e}", kits.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path().join("x64")))
        .filter(|dir| dir.join("makeappx.exe").exists())
        .collect::<Vec<_>>();

    // Folder names are versions like 10.0.26100.0; compare them numerically.
    versions.sort_by_key(|dir| {
        dir.parent()
            .and_then(|dir| dir.file_name())
            .map(|name| {
                name.to_string_lossy()
                    .split('.')
                    .map(|part| part.parse::<u32>().unwrap_or(0))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });

    versions
        .pop()
        .unwrap_or_else(|| panic!("No Windows SDK with makeappx.exe in {}", kits.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_takes_alvr_version_and_hours() {
        let version = msix_version("21.0.0-dev12");
        let (base, build) = version.rsplit_once('.').unwrap();
        assert_eq!(base, "21.0.0");

        // November 2026 or later.
        assert!(build.parse::<u16>().unwrap() >= 7272, "{version}");
    }

    #[test]
    fn full_version_is_kept() {
        assert_eq!(msix_version("21.0.0.5"), "21.0.0.5");
    }
}
