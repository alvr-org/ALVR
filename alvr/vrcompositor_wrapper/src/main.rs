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

    // https://github.com/KhronosGroup/Vulkan-Loader/blob/256a5e3b6d6fc31e711f912291498becd6a41330/docs/LoaderApplicationInterface.md#forcing-layer-source-folders
    let sep = if cfg!(windows) { ";" } else { ":" };
    std::env::set_var(
        "VK_LAYER_PATH",
        if let Ok(existing) = std::env::var("VK_LAYER_PATH") {
            format!("{}{sep}{}", layer_path.to_str().unwrap(), existing)
        } else {
            layer_path.to_str().unwrap().to_owned()
        },
    );

    if std::env::var("VK_APIDUMP_LOG_FILENAME").is_err() {
        // With this patch by Xytovl, you can also use the %n and %p format strings
        // for the executable name and PID respectively.
        //
        // https://gist.githubusercontent.com/ckiee/038809f55f658595107b2da41acff298/raw/6d8d0a91bfd335a25e88cc76eec5c22bf1ece611/vulkantools-log.patch
        std::env::set_var(
            "VK_APIDUMP_LOG_FILENAME",
            "/tmp/alvr_vk_apidump_vrcompositor",
        );
    }

    {
        let debug_instance_layers = if std::env::var("ALVR_VK_DEBUG").is_ok() {
            eprintln!(
                "ALVR_VK_DEBUG is on! Log path is {:?}",
                std::env::var("VK_APIDUMP_LOG_FILENAME")
            );
            format!("VK_LAYER_LUNARG_api_dump{sep}VK_LAYER_KHRONOS_validation{sep}")
        } else {
            "".to_string()
        };
        std::env::set_var(
            "VK_INSTANCE_LAYERS",
            format!("{debug_instance_layers}VK_LAYER_ALVR_capture"),
        );
    }

    // fossilize breaks the ALVR vulkan layer
    std::env::set_var("DISABLE_VK_LAYER_VALVE_steam_fossilize_1", "1");

    let err = exec::execvp(argv0 + ".real", std::env::args());
    println!("Failed to run vrcompositor {err}");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    unimplemented!();
}
