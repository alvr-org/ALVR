use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_linux";
#[cfg(windows)]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_windows";
#[cfg(target_os = "macos")]
const SERVER_BUILD_DIR_NAME: &str = "alvr_server_macos";

#[cfg(not(windows))]
pub fn exec_fname(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
pub fn exec_fname(name: &str) -> String {
    format!("{}.exe", name)
}

#[cfg(target_os = "linux")]
pub fn dynlib_fname(name: &str) -> String {
    format!("lib{}.so", name)
}
#[cfg(windows)]
pub fn dynlib_fname(name: &str) -> String {
    format!("{}.dll", name)
}
#[cfg(target_os = "macos")]
pub fn dynlib_fname(name: &str) -> String {
    format!("lib{}.dylib", name)
}

pub fn target_dir() -> PathBuf {
    Path::new(env!("OUT_DIR")).join("../../../..")
}

pub fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into()
}

pub fn deps_dir() -> PathBuf {
    workspace_dir().join("deps")
}

pub fn build_dir() -> PathBuf {
    workspace_dir().join("build")
}

pub fn server_build_dir() -> PathBuf {
    build_dir().join(SERVER_BUILD_DIR_NAME)
}

// Layout of the ALVR installation. All paths are absolute
pub struct Layout {
    // directory containing the launcher executable
    pub user_executables_dir: PathBuf,
    // parent directory of resources like the dashboard and presets folders
    pub static_resources_dir: PathBuf,
    // directory for storing configuration files (session.json)
    pub config_dir: PathBuf,
    // directory for storing log
    pub log_dir: PathBuf,
    // directory to register in openVR driver path
    pub openvr_driver_dir: PathBuf,
    // (linux only) parent directory of the executable to wrap vrcompositor
    pub vrcompositor_wrapper_dir: PathBuf,
}

impl Layout {
    // This constructor should be used directly only for development builds and packaging
    pub fn from_common_ancestor(path: &Path) -> Self {
        if cfg!(any(windows, target_os = "macos")) {
            Self {
                user_executables_dir: path.to_owned(),
                static_resources_dir: path.to_owned(),
                config_dir: path.to_owned(),
                log_dir: path.to_owned(),
                openvr_driver_dir: path.to_owned(),
                vrcompositor_wrapper_dir: PathBuf::new(),
            }
        } else if cfg!(target_os = "linux") {
            Self {
                user_executables_dir: path.join("bin"),
                static_resources_dir: path.join("share/alvr"),
                config_dir: dirs::config_dir().unwrap().join("alvr"),
                log_dir: dirs::home_dir().unwrap(),
                openvr_driver_dir: path.join("lib64/alvr"),
                vrcompositor_wrapper_dir: path.join("libexec/alvr"),
            }
        } else {
            unimplemented!()
        }
    }

    // The path should include the executable file name
    pub fn from_launcher_exe(path: &Path) -> Self {
        let ancestor = if cfg!(any(windows, target_os = "macos")) {
            path.parent().unwrap()
        } else if cfg!(target_os = "linux") {
            path.parent().unwrap().parent().unwrap()
        } else {
            unimplemented!()
        };

        Self::from_common_ancestor(ancestor)
    }

    pub fn from_openvr_driver_dir(dir: &Path) -> Self {
        let ancestor = if cfg!(windows) || cfg!(target_os = "macos") {
            dir
        } else if cfg!(target_os = "linux") {
            dir.parent().unwrap().parent().unwrap()
        } else {
            unimplemented!()
        };

        Self::from_common_ancestor(ancestor)
    }

    pub fn from_default_installation() -> Self {
        let ancestor = if cfg!(windows) {
            PathBuf::from(r"C:\Program Files\ALVR")
        } else if cfg!(target_os = "linux") {
            PathBuf::from("/")
        } else if cfg!(target_os = "macos") {
            env::temp_dir()
        } else {
            unimplemented!()
        };

        Self::from_common_ancestor(&ancestor)
    }

    pub fn launcher_exe(&self) -> PathBuf {
        let exe = if cfg!(windows) {
            "ALVR Launcher.exe"
        } else if cfg!(any(target_os = "linux", target_os = "macos")) {
            "alvr_launcher"
        } else {
            unimplemented!()
        };
        self.user_executables_dir.join(exe)
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
        if cfg!(windows) {
            self.log_dir.join("session_log.txt")
        } else {
            self.log_dir.join("alvr_session_log.txt")
        }
    }

    pub fn crash_log(&self) -> PathBuf {
        self.log_dir.join("crash_log.txt")
    }

    // path to the shared library to be loaded by openVR
    pub fn openvr_driver_lib(&self) -> PathBuf {
        if cfg!(windows) {
            self.openvr_driver_dir
                .join("bin/win64/driver_alvr_server.dll")
        } else if cfg!(target_os = "linux") {
            self.openvr_driver_dir
                .join("bin/linux64/driver_alvr_server.so")
        } else if cfg!(target_os = "macos") {
            self.openvr_driver_dir
                .join("bin/macos/driver_alvr_server.dylib")
        } else {
            unimplemented!()
        }
    }

    // path to the manifest file for openVR
    pub fn openvr_driver_manifest(&self) -> PathBuf {
        self.openvr_driver_dir.join("driver.vrdrivermanifest")
    }

    pub fn vrcompositor_wrapper(&self) -> PathBuf {
        self.vrcompositor_wrapper_dir.join("vrcompositor-wrapper")
    }
}
