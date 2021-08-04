use std::env;

fn main() {
    // Convert build environment variables into compile-time variables

    // Used only for Linux. These paths should be absolute and override the installation type
    println!(
        "cargo:rustc-env=executables_dir={}",
        env::var("ALVR_EXECUTABLES_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=static_resources_dir={}",
        env::var("ALVR_STATIC_RESOURCES_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=config_dir={}",
        env::var("ALVR_CONFIG_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=log_dir={}",
        env::var("ALVR_LOG_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=openvr_driver_dir={}",
        env::var("ALVR_OPENVR_DRIVER_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=vrcompositor_wrapper_dir={}",
        env::var("ALVR_VRCOMPOSITOR_WRAPPER_DIR").unwrap_or_else(|_| "".to_owned())
    );

    // Used only when custom-root feature is enabled
    println!(
        "cargo:rustc-env=custom_root={}",
        env::var("ALVR_ROOT_DIR").unwrap_or_else(|_| "".to_owned())
    );
}
