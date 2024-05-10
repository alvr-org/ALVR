use std::fs;
use std::{env, process::Command};

use alvr_common::anyhow::bail;
use alvr_common::{debug, error, info, warn};
use sysinfo::Process;

pub fn start_steamvr() {
    Command::new("steam")
        .args(["steam://rungameid/250820"])
        .spawn()
        .ok();
}

pub fn terminate_process(process: &Process) {
    process.kill_with(sysinfo::Signal::Term);
}

pub fn maybe_wrap_vrcompositor_launcher() -> alvr_common::anyhow::Result<()> {
    let steamvr_bin_dir = alvr_server_io::steamvr_root_dir()?
        .join("bin")
        .join("linux64");
    let steamvr_vrserver_path = steamvr_bin_dir.join("vrserver");
    debug!(
        "File path used to check for linux files: {}",
        steamvr_vrserver_path.display().to_string()
    );
    match steamvr_vrserver_path.try_exists() {
        Ok(exists) => {
            if !exists {
                bail!(
                    "SteamVR linux files missing, aborting startup, please re-check compatibility tools for SteamVR or verify integrity of files for SteamVR."
                );
            }
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    let launcher_path = steamvr_bin_dir.join("vrcompositor");
    // In case of SteamVR update, vrcompositor will be restored
    if fs::read_link(&launcher_path).is_ok() {
        fs::remove_file(&launcher_path)?; // recreate the link
    } else {
        fs::rename(&launcher_path, steamvr_bin_dir.join("vrcompositor.real"))?;
    }

    std::os::unix::fs::symlink(
        alvr_filesystem::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
            .vrcompositor_wrapper(),
        &launcher_path,
    )?;

    Ok(())
}
#[derive(PartialEq)]
enum DeviceInfo {
    Nvidia,
    Amd { device_type: wgpu::DeviceType },
    Intel { device_type: wgpu::DeviceType },
    Unknown,
}

pub fn linux_hardware_checks() {
    let wgpu_adapters = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        flags: wgpu::InstanceFlags::empty(),
        dx12_shader_compiler: Default::default(),
        gles_minor_version: Default::default(),
    })
    .enumerate_adapters(wgpu::Backends::VULKAN);
    let device_infos = wgpu_adapters
        .iter()
        .filter(|adapter| {
            adapter.get_info().device_type == wgpu::DeviceType::DiscreteGpu
                || adapter.get_info().device_type == wgpu::DeviceType::IntegratedGpu
        })
        .map(|adapter| match adapter.get_info().vendor {
            0x10de => DeviceInfo::Nvidia,
            0x1002 => DeviceInfo::Amd {
                device_type: adapter.get_info().device_type,
            },
            0x8086 => DeviceInfo::Intel {
                device_type: adapter.get_info().device_type,
            },
            _ => DeviceInfo::Unknown,
        })
        .collect::<Vec<_>>();
    linux_hybrid_gpu_checks(&device_infos);
    linux_encoder_checks(&device_infos);
}

fn linux_hybrid_gpu_checks(device_infos: &[DeviceInfo]) {
    let have_igpu = device_infos.iter().any(|gpu| {
        gpu == &DeviceInfo::Amd {
            device_type: wgpu::DeviceType::IntegratedGpu,
        } || gpu
            == &DeviceInfo::Intel {
                device_type: wgpu::DeviceType::IntegratedGpu,
            }
    });
    debug!("have_igpu: {}", have_igpu);
    let have_nvidia_dgpu = device_infos.iter().any(|gpu| gpu == &DeviceInfo::Nvidia);
    debug!("have_nvidia_dgpu: {}", have_nvidia_dgpu);
    let have_amd_dgpu = device_infos.iter().any(|gpu| {
        gpu == &DeviceInfo::Amd {
            device_type: wgpu::DeviceType::DiscreteGpu,
        }
    });
    debug!("have_amd_dgpu: {}", have_amd_dgpu);
    let have_intel_dgpu = device_infos.iter().any(|gpu| {
        gpu == &DeviceInfo::Intel {
            device_type: wgpu::DeviceType::DiscreteGpu,
        }
    });
    debug!("have_intel_dgpu: {}", have_intel_dgpu);
    if have_igpu {
        if have_nvidia_dgpu {
            warn!("For functioning VR you need insert following into SteamVR and ALL (!) games commandline options:");
            warn!("__GLX_VENDOR_LIBRARY_NAME=nvidia __NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only \
            VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json %command%")
        } else if have_intel_dgpu || have_amd_dgpu {
            warn!("For functioning VR you need insert following into SteamVR and ALL (!) games commandline options:");
            warn!("DRI_PRIME=1 %command%")
        }
    }
}
fn linux_encoder_checks(device_infos: &[DeviceInfo]) {
    for device_info in device_infos {
        match device_info {
            DeviceInfo::Nvidia => {
                match nvml_wrapper::Nvml::init() {
                    Ok(nvml) => {
                        let device_count = nvml.device_count().unwrap();
                        debug!("nvml device count: {}", device_count);
                        // fixme: on multi-gpu nvidia system will do it twice,
                        for index in 0..device_count {
                            match nvml.device_by_index(index) {
                                Ok(device) => {
                                    debug!("nvml device name: {}", device.name().unwrap());
                                    probe_nvenc_encoder_profile(
                                        &device,
                                        nvml_wrapper::enum_wrappers::device::EncoderType::H264,
                                        "H264",
                                    );
                                    probe_nvenc_encoder_profile(
                                        &device,
                                        nvml_wrapper::enum_wrappers::device::EncoderType::HEVC,
                                        "HEVC",
                                    );
                                    // todo: probe for AV1 when will be available in nvml-wrapper
                                }
                                Err(e) => {
                                    error!("Failed to acquire NVML device with error: {}", e)
                                }
                            }
                        }
                    }
                    Err(e) => {
                        alvr_common::show_e(format!("Can't initialize NVML engine, error: {}.", e))
                    }
                }
            }
            DeviceInfo::Amd { device_type: _ } | DeviceInfo::Intel { device_type: _ } => {
                let libva_display_open = libva::Display::open();
                if let Some(libva_display) = libva_display_open {
                    if let Ok(vendor_string) = libva_display.query_vendor_string() {
                        info!("GPU Encoder vendor: {}", vendor_string);
                    }
                    probe_libva_encoder_profile(
                        &libva_display,
                        libva::VAProfile::VAProfileH264Main,
                        "H264",
                        true,
                    );
                    probe_libva_encoder_profile(
                        &libva_display,
                        libva::VAProfile::VAProfileHEVCMain,
                        "HEVC",
                        true,
                    );
                    probe_libva_encoder_profile(
                        &libva_display,
                        libva::VAProfile::VAProfileAV1Profile0,
                        "AV1",
                        false,
                    );
                } else {
                    alvr_common::show_e(
                        "Couldn't find VA-API runtime on system, \
                        you unlikely to have hardware encoding. \
                        Please install VA-API runtime for your distribution \
                        and make sure it works (Manjaro, Fedora).",
                    );
                }
            }
            _ => alvr_common::show_e(
                "Couldn't determine gpu for hardware encoding. \
            You will likely fallback to software encoding.",
            ),
        }
    }
}

fn probe_nvenc_encoder_profile(
    device: &nvml_wrapper::Device,
    encoder_type: nvml_wrapper::enum_wrappers::device::EncoderType,
    profile_name: &str,
) {
    match device.encoder_capacity(encoder_type) {
        Ok(_) => {
            info!("GPU supports {} profile.", profile_name);
        }
        Err(e) => match e {
            nvml_wrapper::error::NvmlError::NotSupported => alvr_common::show_e(format!(
                "Your NVIDIA gpu doesn't support {}. Please make sure CUDA is installed properly. Error: {}",
                profile_name, e
            )),
            _ => error!("{}", e),
        },
    }
}

fn probe_libva_encoder_profile(
    libva_display: &std::rc::Rc<libva::Display>,
    profile_type: libva::VAProfile::Type,
    profile_name: &str,
    is_critical: bool,
) {
    let profile_probe = libva_display.query_config_entrypoints(profile_type);
    let mut message = String::new();
    if profile_probe.is_err() {
        message = format!(
            "Couldn't find {} profile. You unlikely to have hardware encoding for it.",
            profile_name
        );
    } else if let Ok(profile) = profile_probe {
        if profile.is_empty() {
            message = format!(
                "{} profile entrypoint is empty. \
                You unlikely to have hardware encoding for it.",
                profile_name
            );
        }
        if !profile.contains(&libva::VAEntrypoint::VAEntrypointEncSlice) {
            message = format!(
                "{} profile does not contain encoding entrypoint. \
                You unlikely to have hardware encoding for it.",
                profile_name
            );
        }
    }
    if !message.is_empty() {
        if is_critical {
            error!("{}", message);
        } else {
            info!("{}", message);
        }
    }
}
