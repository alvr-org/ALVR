use std::fs;
use std::path::Path;
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
                    "SteamVR Linux files missing, aborting startup, please re-check compatibility tools for SteamVR, verify integrity of files for SteamVR and make sure you're not using Flatpak Steam with non-Flatpak ALVR."
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
    let wgpu_adapters = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    })
    .enumerate_adapters(wgpu::Backends::VULKAN);
    let device_infos = wgpu_adapters
        .iter()
        .filter(|adapter| {
            adapter.get_info().device_type == wgpu::DeviceType::DiscreteGpu
                || adapter.get_info().device_type == wgpu::DeviceType::IntegratedGpu
        })
        .map(|adapter| {
            let vendor = match adapter.get_info().vendor {
                0x10de => DeviceInfo::Nvidia,
                0x1002 => DeviceInfo::Amd {
                    device_type: adapter.get_info().device_type,
                },
                0x8086 => DeviceInfo::Intel {
                    device_type: adapter.get_info().device_type,
                },
                _ => DeviceInfo::Unknown,
            };

            (adapter, vendor)
        })
        .collect::<Vec<_>>();
    linux_gpu_checks(&device_infos);
    linux_encoder_checks(&device_infos);
}

fn linux_gpu_checks(device_infos: &[(&wgpu::Adapter, DeviceInfo)]) {
    let have_intel_igpu = device_infos.iter().any(|gpu| {
        gpu.1
            == DeviceInfo::Intel {
                device_type: wgpu::DeviceType::IntegratedGpu,
            }
    });
    debug!("have_intel_igpu: {}", have_intel_igpu);
    let have_amd_igpu = device_infos.iter().any(|gpu| {
        gpu.1
            == DeviceInfo::Amd {
                device_type: wgpu::DeviceType::IntegratedGpu,
            }
    });
    debug!("have_amd_igpu: {}", have_amd_igpu);

    let have_igpu = have_intel_igpu || have_amd_igpu;
    debug!("have_igpu: {}", have_igpu);

    let have_nvidia_dgpu = device_infos.iter().any(|gpu| gpu.1 == DeviceInfo::Nvidia);
    debug!("have_nvidia_dgpu: {}", have_nvidia_dgpu);

    let have_amd_dgpu = device_infos.iter().any(|gpu| {
        gpu.1
            == DeviceInfo::Amd {
                device_type: wgpu::DeviceType::DiscreteGpu,
            }
    });
    debug!("have_amd_dgpu: {}", have_amd_dgpu);

    if have_amd_igpu || have_amd_dgpu {
        let is_any_amd_driver_invalid = device_infos.iter().any(|gpu| {
            info!("Driver name: {}", gpu.0.get_info().driver);
            match gpu.0.get_info().driver.as_str() {
                "AMD proprietary driver" | "AMD open-source driver" => true, // AMDGPU-Pro | AMDVLK
                _ => false,
            }
        });
        if is_any_amd_driver_invalid {
            error!("Amdvlk or amdgpu-pro vulkan drivers detected, SteamVR may not function properly. \
            Please remove them or make them unavailable for SteamVR and games you're trying to launch.\n\
            For more detailed info visit the wiki: \
            https://github.com/alvr-org/ALVR/wiki/Linux-Troubleshooting#artifacting-no-steamvr-overlay-or-graphical-glitches-in-streaming-view")
        }
    }

    let have_intel_dgpu = device_infos.iter().any(|gpu| {
        gpu.1
            == DeviceInfo::Intel {
                device_type: wgpu::DeviceType::DiscreteGpu,
            }
    });
    debug!("have_intel_dgpu: {}", have_intel_dgpu);

    let steamvr_root_dir = match alvr_server_io::steamvr_root_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Couldn't detect openvr or steamvr files. \
            Please make sure you have installed and ran SteamVR at least once. \
            Or if you're using Flatpak Steam, make sure to use ALVR Dashboard from Flatpak ALVR. {e}");
            return;
        }
    };

    let vrmonitor_path_string = steamvr_root_dir
        .join("bin")
        .join("vrmonitor.sh")
        .into_os_string()
        .into_string()
        .unwrap();
    debug!("vrmonitor_path: {}", vrmonitor_path_string);

    let steamvr_opts = "For functioning VR you need to put the following line into SteamVR's launch options and restart it:";
    let game_opts = "And this similar line to the launch options of ALL games that you're trying to launch from steam:";

    let mut vrmonitor_path_written = false;
    if have_igpu {
        if have_nvidia_dgpu {
            let base_path = "/usr/share/vulkan/icd.d/nvidia_icd";
            let nvidia_vk_override_path = if Path::new(&format!("{base_path}.json")).exists() {
                format!("VK_DRIVER_FILES={base_path}.json")
            } else if Path::new(&format!("{base_path}.x86_64.json")).exists() {
                format!("VK_DRIVER_FILES={base_path}.x86_64.json")
            } else {
                "__VK_LAYER_NV_optimus=NVIDIA_only".to_string()
            };
            let nv_options = format!("__GLX_VENDOR_LIBRARY_NAME=nvidia __NV_PRIME_RENDER_OFFLOAD=1 {nvidia_vk_override_path}");

            warn!("{steamvr_opts}\n{nv_options} {vrmonitor_path_string} %command%");
            warn!("{game_opts}\n{nv_options} %command%");

            vrmonitor_path_written = true;
        } else if have_intel_dgpu || have_amd_dgpu {
            warn!("{steamvr_opts}\nDRI_PRIME=1 {vrmonitor_path_string} %command%");
            warn!("{game_opts}\nDRI_PRIME=1 %command%");
            vrmonitor_path_written = true;
        } else {
            warn!("Beware, using just integrated graphics might lead to very poor performance in SteamVR and VR games.");
            warn!("For more information, please refer to the wiki: https://github.com/alvr-org/ALVR/wiki/Linux-Troubleshooting")
        }
    }
    if !vrmonitor_path_written {
        warn!(
            "Make sure you have put the following line in your SteamVR launch options and restart it:\n\
            {vrmonitor_path_string} %command%"
        )
    }
}

fn linux_encoder_checks(device_infos: &[(&wgpu::Adapter, DeviceInfo)]) {
    for device_info in device_infos {
        match device_info.1 {
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
                        and make sure it works (Manjaro, Fedora affected). \
                        For detailed advice, check wiki: \
                        https://github.com/alvr-org/ALVR/wiki/Linux-Troubleshooting#failed-to-create-vaapi-encoder",
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
        message = format!("Couldn't find {} encoder.", profile_name);
    } else if let Ok(profile) = profile_probe {
        if profile.is_empty() {
            message = format!("{} profile entrypoint is empty.", profile_name);
        }
        if !profile.contains(&libva::VAEntrypoint::VAEntrypointEncSlice) {
            message = format!(
                "{} profile does not contain encoding entrypoint.",
                profile_name
            );
        }
    }
    if !message.is_empty() {
        if is_critical {
            error!("{} Your gpu may not suport encoding with this.", message);
        } else {
            info!(
                "{}
                Your gpu may not suport encoding with this. \
            If you're not using this encoder, ignore this message.",
                message
            );
        }
    }
}
