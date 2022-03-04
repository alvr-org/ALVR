use alvr_common::prelude::*;
use serde_json as json;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::PathBuf,
};

const DRIVER_PATHS_BACKUP_FNAME: &str = "alvr_drivers_paths_backup.txt";

pub fn get_registered_drivers() -> StrResult<Vec<PathBuf>> {
    Ok(crate::from_openvr_paths(
        crate::load_openvr_paths_json()?
            .get_mut("external_drivers")
            .ok_or_else(enone!())?,
    ))
}

pub fn driver_registration(driver_paths: &[PathBuf], register: bool) -> StrResult {
    let mut openvr_paths_json = crate::load_openvr_paths_json()?;
    let paths_json_ref = openvr_paths_json
        .get_mut("external_drivers")
        .ok_or_else(enone!())?;

    let mut paths: HashSet<_> = crate::from_openvr_paths(paths_json_ref)
        .into_iter()
        .collect();

    if register {
        paths.extend(driver_paths.iter().cloned());
    } else {
        for path in driver_paths {
            paths.remove(path);
        }
    }

    // write into openvr_paths_json, the other fields are preserved
    *paths_json_ref = crate::to_openvr_paths(paths.into_iter().collect::<Vec<_>>().as_slice());

    crate::save_openvr_paths_json(&openvr_paths_json)
}

pub fn get_driver_dir_from_registered() -> StrResult<PathBuf> {
    for dir in get_registered_drivers()? {
        let maybe_driver_name = || -> StrResult<_> {
            let manifest_string =
                fs::read_to_string(dir.join("driver.vrdrivermanifest")).map_err(err!())?;
            let mut manifest_map =
                json::from_str::<HashMap<String, json::Value>>(&manifest_string).map_err(err!())?;

            manifest_map.remove("name").ok_or_else(enone!())
        }();

        if maybe_driver_name == Ok(json::Value::String("alvr_server".to_owned())) {
            return Ok(dir);
        }
    }
    fmt_e!("ALVR driver path not registered")
}

pub fn get_driver_dir() -> StrResult<PathBuf> {
    get_driver_dir_from_registered()
        .map_err(|e| format!("ALVR driver path not stored and not registered ({e})"))
}

fn driver_paths_backup_present() -> bool {
    env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME).exists()
}

pub fn apply_driver_paths_backup(driver_dir: PathBuf) -> StrResult {
    if driver_paths_backup_present() {
        let backup_path = env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME);
        let driver_paths =
            json::from_str::<Vec<_>>(&fs::read_to_string(&backup_path).map_err(err!())?)
                .map_err(err!())?;
        fs::remove_file(backup_path).map_err(err!())?;

        driver_registration(&[driver_dir], false)?;

        driver_registration(&driver_paths, true).ok();
    }

    Ok(())
}

pub fn maybe_save_driver_paths_backup(paths_backup: &[PathBuf]) -> StrResult {
    if !driver_paths_backup_present() {
        fs::write(
            env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME),
            json::to_string_pretty(paths_backup).map_err(err!())?,
        )
        .map_err(err!())?;
    }

    Ok(())
}
