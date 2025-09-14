use crate::{
    command,
    dependencies::{OpenXRLoadersSelection, windows::choco_install},
};
use alvr_filesystem as afs;
use std::fs;
use xshell::{Shell, cmd};

pub fn build_deps(
    skip_admin_priv: bool,
    all_targets: bool,
    openxr_loaders_selection: OpenXRLoadersSelection,
) {
    let sh = Shell::new().unwrap();

    if cfg!(windows) && !skip_admin_priv {
        choco_install(&sh, &["unzip", "llvm"]).unwrap();
    }

    cmd!(sh, "rustup target add aarch64-linux-android")
        .run()
        .unwrap();
    if all_targets {
        cmd!(sh, "rustup target add armv7-linux-androideabi")
            .run()
            .unwrap();
        cmd!(sh, "rustup target add x86_64-linux-android")
            .run()
            .unwrap();
        cmd!(sh, "rustup target add i686-linux-android")
            .run()
            .unwrap();
    }
    cmd!(sh, "cargo install cbindgen").run().unwrap();
    cmd!(sh, "cargo install cargo-ndk --version 3.5.4")
        .run()
        .unwrap();
    cmd!(
        sh,
        "cargo install --git https://github.com/zarik5/cargo-apk cargo-apk"
    )
    .run()
    .unwrap();

    get_android_openxr_loaders(openxr_loaders_selection);
}

fn get_android_openxr_loaders(selection: OpenXRLoadersSelection) {
    fn get_openxr_loader(name: &str, url: &str, source_dir: &str) {
        let sh = Shell::new().unwrap();
        let temp_dir = afs::build_dir().join("temp_download");
        sh.remove_path(&temp_dir).ok();
        sh.create_dir(&temp_dir).unwrap();
        let destination_dir = afs::deps_dir().join("android_openxr/arm64-v8a");
        fs::create_dir_all(&destination_dir).unwrap();

        command::download_and_extract_zip(url, &temp_dir).unwrap();
        fs::copy(
            temp_dir.join(source_dir).join("libopenxr_loader.so"),
            destination_dir.join(format!("libopenxr_loader{name}.so")),
        )
        .unwrap();
        fs::remove_dir_all(&temp_dir).ok();
    }

    get_openxr_loader(
        "",
        &format!(
            "https://github.com/KhronosGroup/OpenXR-SDK-Source/releases/download/{}",
            "release-1.0.34/openxr_loader_for_android-1.0.34.aar",
        ),
        "prefab/modules/openxr_loader/libs/android.arm64-v8a",
    );

    if matches!(selection, OpenXRLoadersSelection::OnlyGeneric) {
        return;
    }

    get_openxr_loader(
        "_pico_old",
        "https://sdk.picovr.com/developer-platform/sdk/PICO_OpenXR_SDK_220.zip",
        "libs/android.arm64-v8a",
    );

    if matches!(selection, OpenXRLoadersSelection::OnlyPico) {
        return;
    }

    get_openxr_loader(
        "_quest1",
        "https://securecdn.oculus.com/binaries/download/?id=7577210995650755", // Version 64
        "OpenXR/Libs/Android/arm64-v8a/Release",
    );

    get_openxr_loader(
        "_yvr",
        "https://developer.yvrdream.com/yvrdoc/sdk/openxr/yvr_openxr_mobile_sdk_2.0.0.zip",
        "yvr_openxr_mobile_sdk_2.0.0/OpenXR/Libs/Android/arm64-v8a",
    );

    get_openxr_loader(
        "_lynx",
        "https://portal.lynx-r.com/downloads/download/16", // version 1.0.0
        "jni/arm64-v8a",
    );
}
