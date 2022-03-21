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
    std::env::set_var("VK_INSTANCE_LAYERS", "VK_LAYER_ALVR_capture");
    // fossilize breaks the ALVR vulkan layer
    std::env::set_var("DISABLE_VK_LAYER_VALVE_steam_fossilize_1", "1");

    let err = exec::execvp(argv0 + ".real", std::env::args());
    println!("Failed to run vrcompositor {err}");
}
