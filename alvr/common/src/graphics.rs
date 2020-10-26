use gfx_hal::Instance;

#[cfg(windows)]
pub fn get_gpu_names() -> Vec<String> {
    let instance = gfx_backend_dx11::Instance::create("ALVR", 0).unwrap();
    let adapters = instance.enumerate_adapters();

    adapters
        .into_iter()
        .map(|a| a.info.name)
        .collect::<Vec<_>>()
}
