use crate::data_sources;
use alvr_common::{debug, glam::bool, once_cell::sync::Lazy, parking_lot::Mutex};
#[cfg(target_os = "linux")]
use alvr_common::{error, info};
use alvr_filesystem as afs;
use alvr_session::{DriverLaunchAction, DriversBackup};
use std::{
    env,
    marker::PhantomData,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_steamvr_running() -> bool {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    system
        .processes_by_name(&afs::exec_fname("vrserver"))
        .count()
        != 0
}

#[cfg(target_os = "linux")]
pub fn maybe_wrap_vrcompositor_launcher() -> alvr_common::anyhow::Result<()> {
    use std::fs;

    use alvr_common::anyhow::bail;

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
        afs::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
            .vrcompositor_wrapper(),
        &launcher_path,
    )?;

    Ok(())
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::os::windows::process::CommandExt;
    Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

pub fn maybe_kill_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.processes_by_name(&afs::exec_fname("vrmonitor")) {
        debug!("Killing vrmonitor");

        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }

    system.refresh_processes();

    for process in system.processes_by_name(&afs::exec_fname("vrserver")) {
        debug!("Killing vrserver");

        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }
}

pub struct Launcher {
    _phantom: PhantomData<()>,
}

impl Launcher {
    pub fn launch_steamvr(&self) {
        #[cfg(target_os = "linux")]
        linux_hardware_encoders_check();

        let mut data_source = data_sources::get_local_data_source();

        let launch_action = &data_source.settings().steamvr_launcher.driver_launch_action;

        if !matches!(launch_action, DriverLaunchAction::NoAction) {
            let other_drivers_paths = if matches!(
                launch_action,
                DriverLaunchAction::UnregisterOtherDriversAtStartup
            ) && data_source.session().drivers_backup.is_none()
            {
                let drivers_paths = alvr_server_io::get_registered_drivers().unwrap_or_default();

                alvr_server_io::driver_registration(&drivers_paths, false).ok();

                drivers_paths
            } else {
                vec![]
            };
            let alvr_driver_dir =
                afs::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
                    .openvr_driver_root_dir;

            alvr_server_io::driver_registration(&[alvr_driver_dir.clone()], true).ok();

            data_source.session_mut().drivers_backup = Some(DriversBackup {
                alvr_path: alvr_driver_dir,
                other_paths: other_drivers_paths,
            });
        }

        #[cfg(target_os = "linux")]
        {
            let vrcompositor_wrap_result = maybe_wrap_vrcompositor_launcher();
            alvr_common::show_err(maybe_wrap_vrcompositor_launcher());
            if let Err(_) = vrcompositor_wrap_result {
                return;
            }
        }

        if !is_steamvr_running() {
            debug!("SteamVR is dead. Launching...");

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                Command::new("cmd")
                    .args(["/C", "start", "steam://rungameid/250820"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .ok();
            }
            #[cfg(not(windows))]
            {
                Command::new("steam")
                    .args(["steam://rungameid/250820"])
                    .spawn()
                    .ok();
            }
        }
    }

    pub fn ensure_steamvr_shutdown(&self) {
        debug!("Waiting for SteamVR to shutdown...");
        let start_time = Instant::now();
        while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
            thread::sleep(Duration::from_millis(500));
        }

        maybe_kill_steamvr();
    }

    pub fn restart_steamvr(&self) {
        self.ensure_steamvr_shutdown();
        self.launch_steamvr();
    }
}

#[cfg(target_os = "linux")]
fn linux_hardware_encoders_check() {
    enum GpuType {
        Nvidia,
        Amd,
        Intel,
        Llvmpipe,
        Unknown,
    }
    let wgpu_adapters = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        flags: wgpu::InstanceFlags::empty(),
        dx12_shader_compiler: Default::default(),
        gles_minor_version: Default::default(),
    })
    .enumerate_adapters(wgpu::Backends::VULKAN);
    let gpu_types = wgpu_adapters
        .iter()
        .map(|adapter| match adapter.get_info().vendor {
            0x10de => {
                return Some(GpuType::Nvidia);
            }
            0x1002 => {
                return Some(GpuType::Amd);
            }
            0x8086 => {
                return Some(GpuType::Intel);
            }
            0x10005 => {
                return Some(GpuType::Llvmpipe);
            }
            _ => {
                return Some(GpuType::Unknown);
            }
        })
        .flatten();
    for gpu_type in gpu_types {
        match gpu_type {
            GpuType::Nvidia => {
                if let Ok(nvml) = nvml_wrapper::Nvml::init() {
                    let device_count = nvml.device_count().unwrap();
                    debug!("device count: {}", device_count);
                    // fixme: on multi-gpu nvidia system will do it twice,
                    for index in 0..device_count {
                        if let Ok(device) = nvml.device_by_index(index) {
                            debug!("device name: {}", device.name().unwrap());
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
                        } else {
                            error!("Failed to initialize CUDA device.")
                        }
                    }
                } else {
                    alvr_common::show_e("Can't probe for nvenc. Please install CUDA.")
                }
            }
            GpuType::Amd | GpuType::Intel => {
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
            GpuType::Unknown => alvr_common::show_e(
                "Couldn't determine gpu for hardware encoding. \
            You will likely fallback to software encoding.",
            ),
            GpuType::Llvmpipe => debug!("Found software vulkan driver."),
        }
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn probe_libva_encoder_profile(
    libva_display: &std::rc::Rc<libva::Display>,
    profile_type: libva::VAProfile::Type,
    profile_name: &str,
    is_critical: bool,
) {
    let profile_probe = libva_display.query_config_entrypoints(profile_type);
    if profile_probe.is_err() {
        let message = format!(
            "Couldn't find {} profile. You unlikely to have hardware encoding for it.",
            profile_name
        );
        if is_critical {
            error!("{}", message);
        } else {
            info!("{}", message);
        }
    } else if let Ok(profile) = profile_probe {
        if profile.is_empty() {
            let message = format!(
                "{} profile entrypoint is empty. \
                You unlikely to have hardware encoding for it.",
                profile_name
            );
            if is_critical {
                error!("{}", message);
            } else {
                info!("{}", message);
            }
        }
        if !profile.contains(&libva::VAEntrypoint::VAEntrypointEncSlice) {
            let message = format!(
                "{} profile does not contain encoding entrypoint. \
                You unlikely to have hardware encoding for it.",
                profile_name
            );
            if is_critical {
                error!("{}", message);
            } else {
                info!("{}", message);
            }
        }
    }
}

// Singleton with exclusive access
pub static LAUNCHER: Lazy<Mutex<Launcher>> = Lazy::new(|| {
    Mutex::new(Launcher {
        _phantom: PhantomData,
    })
});
