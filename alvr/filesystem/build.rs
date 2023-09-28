use std::env;

fn print_env(build_env: &str, compile_env: &str) {
    println!("cargo:rerun-if-env-changed={}", build_env);
    if let Ok(var) = env::var(build_env) {
        println!("cargo:rustc-env={}={}", compile_env, var);
    }
}

fn main() {
    // Convert build environment variables into compile-time variables

    // Used only for Linux. These paths should be relative to the root/prefix and override the installation
    print_env("ALVR_EXECUTABLES_DIR", "executables_dir");
    print_env("ALVR_LIBRARIES_DIR", "libraries_dir");
    print_env("ALVR_STATIC_RESOURCES_DIR", "static_resources_dir");
    print_env("ALVR_OPENVR_DRIVER_ROOT_DIR", "openvr_driver_root_dir");
    print_env("ALVR_VRCOMPOSITOR_WRAPPER_DIR", "vrcompositor_wrapper_dir");
    print_env("FIREWALL_SCRIPT_DIR", "firewall_script_dir");
    print_env("FIREWALLD_CONFIG_DIR", "firewalld_config_dir");
    print_env("UFW_CONFIG_DIR", "ufw_config_dir");
    print_env(
        "ALVR_VULKAN_LAYER_MANIFEST_DIR",
        "vulkan_layer_manifest_dir",
    );

    // Absolute paths, not based on root
    print_env("ALVR_CONFIG_DIR", "config_dir");
    print_env("ALVR_LOG_DIR", "log_dir");

    // Absolute root dir/prefix path
    print_env("ALVR_ROOT_DIR", "root");
}
