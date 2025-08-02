use std::{
    env::{
        self,
        consts::{DLL_EXTENSION, DLL_PREFIX, DLL_SUFFIX, EXE_SUFFIX, OS},
    },
    path::{Path, PathBuf},
};

pub fn exec_fname(name: &str) -> String {
    format!("{name}{EXE_SUFFIX}")
}

pub fn dynlib_fname(name: &str) -> String {
    format!("{DLL_PREFIX}{name}{DLL_SUFFIX}")
}

pub fn target_dir() -> PathBuf {
    // use `.parent().unwrap()` instead of `../` to maintain canonicalized form
    Path::new(env!("OUT_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

pub fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

pub fn crate_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(name)
}

pub fn deps_dir() -> PathBuf {
    workspace_dir().join("deps")
}

pub fn build_dir() -> PathBuf {
    workspace_dir().join("build")
}

pub fn streamer_build_dir() -> PathBuf {
    build_dir().join(format!("alvr_streamer_{OS}"))
}

pub fn launcher_fname() -> String {
    exec_fname("ALVR Launcher")
}

pub fn launcher_build_dir() -> PathBuf {
    build_dir().join(format!("alvr_launcher_{OS}"))
}

pub fn launcher_build_exe_path() -> PathBuf {
    launcher_build_dir().join(launcher_fname())
}

pub fn installer_path() -> PathBuf {
    env::temp_dir().join(exec_fname("alvr_installer"))
}

pub fn dashboard_fname() -> &'static str {
    if cfg!(windows) {
        "ALVR Dashboard.exe"
    } else {
        "alvr_dashboard"
    }
}

// Layout of the ALVR installation. All paths are absolute
#[derive(Clone, Default, Debug)]
pub struct Layout {
    // directory containing the dashboard executable
    pub executables_dir: PathBuf,
    // parent directory of resources like the dashboard and presets folders
    pub static_resources_dir: PathBuf,
    // directory for storing configuration files (session.json)
    pub config_dir: PathBuf,
    // directory for storing log
    pub log_dir: PathBuf,
    // directory to register in openVR driver path
    pub openvr_driver_root_dir: PathBuf,
    // (linux only) parent directory of the firewall script
    pub firewall_script_dir: PathBuf,
    // (linux only) parent directory of the firewalld config
    pub firewalld_config_dir: PathBuf,
    // (linux only) parent directory of the ufw config
    pub ufw_config_dir: PathBuf,
    pub launcher_root: Option<PathBuf>,
}

impl Layout {
    pub fn new(root: &Path) -> Self {
        #[cfg(target_os = "linux")]
        {
            let or_path =
                |opt: Option<&'static str>, path| opt.map_or(root.join(path), PathBuf::from);

            // Get paths from environment or use FHS compliant paths
            let executables_dir = or_path(option_env!("ALVR_EXECUTABLES_DIR"), "bin");
            let static_resources_dir =
                or_path(option_env!("ALVR_STATIC_RESOURCES_DIR"), "share/alvr");
            let openvr_driver_root_dir =
                or_path(option_env!("ALVR_OPENVR_DRIVER_ROOT_DIR"), "lib64/alvr");
            let firewall_script_dir = or_path(option_env!("FIREWALL_SCRIPT_DIR"), "libexec/alvr");
            let firewalld_config_dir = or_path(option_env!("FIREWALLD_CONFIG_DIR"), "libexec/alvr");
            let ufw_config_dir = or_path(option_env!("UFW_CONFIG_DIR"), "libexec/alvr");

            let config_dir = option_env!("ALVR_CONFIG_DIR")
                .map_or_else(|| dirs::config_dir().unwrap().join("alvr"), PathBuf::from);
            let log_dir = option_env!("ALVR_LOG_DIR")
                .map_or_else(|| dirs::home_dir().unwrap(), PathBuf::from);

            Self {
                executables_dir,
                static_resources_dir,
                config_dir,
                log_dir,
                openvr_driver_root_dir,
                firewall_script_dir,
                firewalld_config_dir,
                ufw_config_dir,
                launcher_root: root
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_owned()),
            }
        }
        #[cfg(not(target_os = "linux"))]
        Self {
            executables_dir: root.to_owned(),
            libraries_dir: root.to_owned(),
            static_resources_dir: root.to_owned(),
            config_dir: root.to_owned(),
            log_dir: root.to_owned(),
            openvr_driver_root_dir: root.to_owned(),
            firewall_script_dir: root.to_owned(),
            firewalld_config_dir: root.to_owned(),
            ufw_config_dir: root.to_owned(),
            launcher_root: root.parent().and_then(|p| p.parent()).map(|p| p.to_owned()),
        }
    }

    pub fn dashboard_exe(&self) -> PathBuf {
        self.executables_dir.join(dashboard_fname())
    }

    pub fn local_adb_exe(&self) -> PathBuf {
        self.executables_dir
            .join("platform-tools")
            .join(exec_fname("adb"))
    }

    pub fn resources_dir(&self) -> PathBuf {
        self.openvr_driver_root_dir.join("resources")
    }

    pub fn dashboard_dir(&self) -> PathBuf {
        self.static_resources_dir.join("dashboard")
    }

    pub fn presets_dir(&self) -> PathBuf {
        self.static_resources_dir.join("presets")
    }

    pub fn session(&self) -> PathBuf {
        self.config_dir.join("session.json")
    }

    pub fn session_log(&self) -> PathBuf {
        if cfg!(target_os = "linux") {
            self.log_dir.join("alvr_session_log.txt")
        } else {
            self.log_dir.join("session_log.txt")
        }
    }

    pub fn connect_script(&self) -> PathBuf {
        self.config_dir.join(if cfg!(windows) {
            "on_connect.bat"
        } else {
            "on_connect.sh"
        })
    }

    pub fn disconnect_script(&self) -> PathBuf {
        self.config_dir.join(if cfg!(windows) {
            "on_disconnect.bat"
        } else {
            "on_disconnect.sh"
        })
    }

    pub fn crash_log(&self) -> PathBuf {
        self.log_dir.join("crash_log.txt")
    }

    pub fn openvr_driver_lib_dir(&self) -> PathBuf {
        let platform = if cfg!(windows) {
            "win64"
        } else if cfg!(target_os = "linux") {
            "linux64"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            unimplemented!()
        };

        self.openvr_driver_root_dir.join("bin").join(platform)
    }

    // path to the shared library to be loaded by openVR
    pub fn openvr_driver_lib(&self) -> PathBuf {
        self.openvr_driver_lib_dir()
            .join(format!("driver_alvr_server.{DLL_EXTENSION}"))
    }

    // path to the manifest file for openVR
    pub fn openvr_driver_manifest(&self) -> PathBuf {
        self.openvr_driver_root_dir.join("driver.vrdrivermanifest")
    }

    pub fn firewall_script(&self) -> PathBuf {
        self.firewall_script_dir.join("alvr_fw_config.sh")
    }

    pub fn firewalld_config(&self) -> PathBuf {
        self.firewalld_config_dir.join("alvr-firewalld.xml")
    }

    pub fn ufw_config(&self) -> PathBuf {
        self.ufw_config_dir.join("ufw-alvr")
    }

    pub fn launcher_exe(&self) -> Option<PathBuf> {
        self.launcher_root
            .as_ref()
            .map(|root| root.join(launcher_fname()))
    }
}

fn layout_from_env() -> Option<Layout> {
    option_env!("ALVR_ROOT_DIR").map(|path| Layout::new(Path::new(path)))
}

// The path should include the executable file name
// The path argument is used only if ALVR is built as portable
pub fn filesystem_layout_from_dashboard_exe(path: &Path) -> Option<Layout> {
    layout_from_env().or_else(|| {
        let root = if cfg!(target_os = "linux") {
            // FHS path is expected
            path.parent()?.parent()?.to_owned()
        } else {
            path.parent()?.to_owned()
        };

        Some(Layout::new(&root))
    })
}

// The dir argument is used only if ALVR is built as portable
pub fn filesystem_layout_from_openvr_driver_root_dir(dir: &Path) -> Option<Layout> {
    layout_from_env().or_else(|| {
        let root = if cfg!(target_os = "linux") {
            // FHS path is expected
            dir.parent()?.parent()?.to_owned()
        } else {
            dir.to_owned()
        };

        Some(Layout::new(&root))
    })
}

// Use this when there is no way of determining the current path. The resulting Layout paths will
// be invalid, except for the ones that disregard the relative path (for example the config dir) and
// the ones that have been overridden.
pub fn filesystem_layout_invalid() -> Layout {
    layout_from_env().unwrap_or_else(|| Layout::new(Path::new("./")))
}
