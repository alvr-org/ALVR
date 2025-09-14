use crate::BuildPlatform;

pub mod android;
pub mod linux;
pub mod windows;

pub enum OpenXRLoadersSelection {
    OnlyGeneric,
    OnlyPico,
    All,
}

pub fn prepare_server_deps(
    platform: Option<BuildPlatform>,
    skip_admin_priv: bool,
    enable_nvenc: bool,
) {
    match platform {
        Some(BuildPlatform::Windows) => windows::prepare_deps(skip_admin_priv),
        Some(BuildPlatform::Linux) => linux::prepare_deps(enable_nvenc),
        Some(BuildPlatform::Macos) => prepare_macos_deps(),
        Some(BuildPlatform::Android) => panic!("Android is not supported"),
        None => {
            if cfg!(windows) {
                windows::prepare_deps(skip_admin_priv);
            } else if cfg!(target_os = "linux") {
                linux::prepare_deps(enable_nvenc);
            } else if cfg!(target_os = "macos") {
                prepare_macos_deps();
            } else {
                panic!("Unsupported platform");
            }
        }
    }
}

pub fn download_server_deps(
    platform: Option<BuildPlatform>,
    skip_admin_priv: bool,
    enable_nvenc: bool,
) {
    match platform {
        Some(BuildPlatform::Windows) => windows::prepare_deps(skip_admin_priv),
        Some(BuildPlatform::Linux) => linux::download_deps(enable_nvenc),
        Some(BuildPlatform::Macos) => prepare_macos_deps(),
        Some(BuildPlatform::Android) => panic!("Android is not supported"),
        None => {
            if cfg!(windows) {
                windows::prepare_deps(skip_admin_priv);
            } else if cfg!(target_os = "linux") {
                linux::download_deps(enable_nvenc);
            } else if cfg!(target_os = "macos") {
                prepare_macos_deps();
            } else {
                panic!("Unsupported platform");
            }
        }
    }
}

pub fn build_server_deps(platform: Option<BuildPlatform>, enable_nvenc: bool) {
    match platform {
        Some(BuildPlatform::Windows) => panic!("Building windows dependencies is unsupported"),
        Some(BuildPlatform::Linux) => linux::build_deps(enable_nvenc),
        Some(BuildPlatform::Macos) => prepare_macos_deps(),
        Some(BuildPlatform::Android) => panic!("Android is not supported"),
        None => {
            if cfg!(windows) {
                panic!("Building windows dependencies is unsupported");
            } else if cfg!(target_os = "linux") {
                linux::build_deps(enable_nvenc);
            } else if cfg!(target_os = "macos") {
                prepare_macos_deps();
            } else {
                panic!("Unsupported platform");
            }
        }
    }
}

pub fn prepare_macos_deps() {}
