use gfx_hal::Instance;

#[derive(serde::Serialize)]
pub struct GpuInfo {
    name: String,
    supported: bool,
}

// Use the vendor and device PCI IDs to determine support
// IDs repository: https://pci-ids.ucw.cz/read/PC
fn is_gpu_supported(vendor_id: usize, device_id: usize, _device_name: String) -> bool {
    // NB: using a "greater than" criterion is a very coarse approximation, because device ids are
    // not ordered by release date or computing power
    match vendor_id {
        // NVIDIA
        // criterion: first GTX TITAN
        0x10de => device_id >= 0x1001,

        // AMD
        // criterion: first RX gpus
        0x1002 => device_id >= 0x67df,

        _ => false,
    }
}

pub fn get_gpus_info() -> Vec<GpuInfo> {
    let instance = gfx_backend_dx11::Instance::create("ALVR", 0).unwrap();
    let adapters = instance.enumerate_adapters();

    adapters
        .into_iter()
        .map(|a| GpuInfo {
            name: a.info.name.clone(),
            supported: is_gpu_supported(a.info.vendor, a.info.device, a.info.name),
        })
        .collect::<Vec<_>>()
}
