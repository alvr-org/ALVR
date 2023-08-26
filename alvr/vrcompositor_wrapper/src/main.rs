#[cfg(target_os = "linux")]
fn main() {
    let argv0 = std::env::args().next().unwrap();
    // location of the ALVR vulkan layer manifest
    let layer_path = match std::fs::read_link(&argv0) {
        Ok(path) => path
            .parent()
            .unwrap()
            .join("../../share/vulkan/explicit_layer.d"),
        Err(err) => panic!("Failed to read vrcompositor symlink: {err}"),
    };
    std::env::set_var("VK_LAYER_PATH", layer_path);
    // Vulkan < 1.3.234
    std::env::set_var("VK_INSTANCE_LAYERS", "VK_LAYER_ALVR_capture");
    std::env::set_var("DISABLE_VK_LAYER_VALVE_steam_fossilize_1", "1");
    std::env::set_var("DISABLE_MANGOHUD", "1");
    std::env::set_var("DISABLE_VKBASALT", "1");
    std::env::set_var("DISABLE_OBS_VKCAPTURE", "1");
    // Vulkan >= 1.3.234
    std::env::set_var(
        "VK_LOADER_LAYERS_ENABLE",
        "VK_LAYER_ALVR_capture,VK_LAYER_MESA_device_select",
    );
    std::env::set_var("VK_LOADER_LAYERS_DISABLE", "*");
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        let drm_lease_shim_path = match std::fs::read_link(&argv0) {
            Ok(path) => path.parent().unwrap().join("alvr_drm_lease_shim.so"),
            Err(err) => panic!("Failed to read vrcompositor symlink: {err}"),
        };
        std::env::set_var("LD_PRELOAD", drm_lease_shim_path);
        std::env::set_var(
            "ALVR_SESSION_JSON",
            alvr_filesystem::filesystem_layout_invalid()
                .session()
                .to_string_lossy()
                .to_string(),
        );
    }

    let err = exec::execvp(argv0 + ".real", std::env::args());
    println!("Failed to run vrcompositor {err}");
}

#[cfg(not(target_os = "linux"))]
fn main() {}
