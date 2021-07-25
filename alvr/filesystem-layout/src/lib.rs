#[macro_use]
extern crate lazy_static;

use std::path::{Path, PathBuf};

// Layout of the ALVR installation, relative path must be interpreted
// as relative to the root of the ALVR installation.
// Either all path must be relative or all absolute.
pub struct Layout {
    // directory to register in openVR driver path
    pub openvr_driver_dir: PathBuf,
    // directory containing default presets
    pub presets_dir: PathBuf,
    // (linux only) path to the executable to wrap vrcompositor
    pub vrcompositor_wrapper: PathBuf,
    // path to the launcher executable
    pub launcher_exe: PathBuf,
    // directory containing dashboard static resources (javascript, css, images)
    pub dashboard_resources_dir: PathBuf,
}

impl Layout {
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
}

lazy_static! {
    pub static ref LAYOUT: Layout = if cfg!(windows) {
        Layout {
            openvr_driver_dir: PathBuf::from(""),
            presets_dir: PathBuf::from("presets"),
            vrcompositor_wrapper: PathBuf::from(""),
            launcher_exe: PathBuf::from("ALVR Launcher.exe"),
            dashboard_resources_dir: PathBuf::from("dashboard"),
        }
    } else if cfg!(target_os = "linux") {
        Layout {
            openvr_driver_dir: PathBuf::from("lib64/alvr"),
            presets_dir: PathBuf::from("share/alvr/presets"),
            vrcompositor_wrapper: PathBuf::from("libexec/alvr/vrcompositor-wrapper"),
            launcher_exe: PathBuf::from("bin/alvr_launcher"),
            dashboard_resources_dir: PathBuf::from("share/alvr/dashboard"),
        }
    } else if cfg!(target_os = "macos") {
        Layout {
            openvr_driver_dir: PathBuf::from(""),
            presets_dir: PathBuf::from("presets"),
            vrcompositor_wrapper: PathBuf::from(""),
            launcher_exe: PathBuf::from("alvr_launcher"),
            dashboard_resources_dir: PathBuf::from("dashboard"),
        }
    } else {
        unimplemented!()
    };
}

pub fn session_log(alvr_dir: &Path) -> PathBuf {
    if cfg!(not(target_os = "linux")) {
        alvr_dir.join("session_log.txt")
    } else {
        dirs::home_dir()
            .expect("get home directory")
            .join("alvr_session_log.txt")
    }
}

pub fn crash_log(alvr_dir: &Path) -> PathBuf {
    if cfg!(not(target_os = "linux")) {
        alvr_dir.join("crash_log.txt")
    } else {
        dirs::home_dir()
            .expect("get home directory")
            .join("alvr_crash_log.txt")
    }
}

// return the base alvr path from the path of one component
pub fn alvr_dir_from_component(
    component_path: &PathBuf,
    component_layout: &PathBuf,
) -> Result<PathBuf, String> {
    if component_layout.is_absolute() {
        return Ok(PathBuf::from("/"));
    }
    if !component_path.ends_with(component_layout) {
        return Err(
            format! {"failed to get alvr dir from {} compared to {}, path do not match",
            component_path.to_string_lossy(),
            component_layout.to_string_lossy()
            },
        );
    }
    let mut res = component_path.clone();
    for _ in component_layout.components() {
        res.pop();
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alvr_dir_from_component() {
        assert_eq!(
            alvr_dir_from_component(
                &PathBuf::from("relative/path/template/part"),
                &PathBuf::from("template/part")
            ),
            Ok(PathBuf::from("relative/path"))
        );

        assert_eq!(
            alvr_dir_from_component(
                &PathBuf::from("relative/path/template/./part"),
                &PathBuf::from("template/part")
            ),
            Ok(PathBuf::from("relative/path"))
        );

        assert_eq!(
            alvr_dir_from_component(
                &PathBuf::from("/absolute/path/template/part"),
                &PathBuf::from("template/part")
            ),
            Ok(PathBuf::from("/absolute/path"))
        );

        assert!(alvr_dir_from_component(
            &PathBuf::from("relative/path/invalid/part"),
            &PathBuf::from("template/part")
        )
        .is_err());

        assert_eq!(
            alvr_dir_from_component(
                &PathBuf::from("absolutely/anything/in/here"),
                &PathBuf::from("/absolute/template")
            ),
            Ok(PathBuf::from("/"))
        );
    }
}
