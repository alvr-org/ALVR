use crate::{BuildPlatform, print_help_and_exit};

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
    if let Some(platform) = platform {
        match platform {
            BuildPlatform::Windows => windows::prepare_deps(skip_admin_priv),
            BuildPlatform::Linux => linux::prepare_deps(enable_nvenc),
            BuildPlatform::Macos => prepare_macos_deps(),
            BuildPlatform::Android => panic!("Android is not supported"),
        }
    } else {
        print_help_and_exit("Please specify platform");
    }
}

pub fn download_server_deps(
    platform: Option<BuildPlatform>,
    skip_admin_priv: bool,
    enable_nvenc: bool,
) {
    if let Some(platform) = platform {
        match platform {
            BuildPlatform::Windows => windows::prepare_deps(skip_admin_priv),
            BuildPlatform::Linux => linux::download_deps(enable_nvenc),
            BuildPlatform::Macos => prepare_macos_deps(),
            BuildPlatform::Android => panic!("Android is not supported"),
        }
    } else {
        print_help_and_exit("Please specify platform");
    }
}

pub fn build_server_deps(platform: Option<BuildPlatform>, enable_nvenc: bool) {
    if let Some(platform) = platform {
        match platform {
            BuildPlatform::Windows => panic!("Building windows dependencies is not supported"),
            BuildPlatform::Linux => linux::build_deps(enable_nvenc),
            BuildPlatform::Macos => prepare_macos_deps(),
            BuildPlatform::Android => panic!("Android is not supported"),
        }
    } else {
        print_help_and_exit("Please specify platform");
    }
}

pub fn prepare_macos_deps() {}
