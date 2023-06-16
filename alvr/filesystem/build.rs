use std::env;

fn main() {
    // Convert build environment variables into compile-time variables

    // Used only for Linux. These paths should be absolute and override the installation type
    println!(
        "cargo:rustc-env=executables_dir={}",
        env::var("ALVR_EXECUTABLES_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=libraries_dir={}",
        env::var("ALVR_LIBRARIES_DIR").unwrap_or_else(|_| "".to_owned())
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
        "cargo:rustc-env=openvr_driver_root_dir={}",
        env::var("ALVR_OPENVR_DRIVER_ROOT_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=vrcompositor_wrapper_dir={}",
        env::var("ALVR_VRCOMPOSITOR_WRAPPER_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=firewall_script_dir={}",
        env::var("FIREWALL_SCRIPT_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=firewalld_config_dir={}",
        env::var("FIREWALLD_CONFIG_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=ufw_config_dir={}",
        env::var("UFW_CONFIG_DIR").unwrap_or_else(|_| "".to_owned())
    );
    println!(
        "cargo:rustc-env=vulkan_layer_manifest_dir={}",
        env::var("ALVR_VULKAN_LAYER_MANIFEST_DIR").unwrap_or_else(|_| "".to_owned())
    );

    println!(
        "cargo:rustc-env=root={}",
        env::var("ALVR_ROOT_DIR").unwrap_or_else(|_| "".to_owned())
    );
}
