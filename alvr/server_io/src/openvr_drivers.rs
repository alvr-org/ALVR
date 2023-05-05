use crate::openvrpaths;
use alvr_common::prelude::*;
use serde_json as json;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

pub fn get_registered_drivers() -> StrResult<Vec<PathBuf>> {
    Ok(openvrpaths::from_openvr_paths(
        openvrpaths::load_openvr_paths_json()?
            .get_mut("external_drivers")
            .ok_or_else(enone!())?,
    ))
}

pub fn driver_registration(driver_paths: &[PathBuf], register: bool) -> StrResult {
    let mut openvr_paths_json = openvrpaths::load_openvr_paths_json()?;
    let paths_json_ref = openvr_paths_json
        .get_mut("external_drivers")
        .ok_or_else(enone!())?;

    let mut paths: HashSet<_> = openvrpaths::from_openvr_paths(paths_json_ref)
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
    *paths_json_ref =
        openvrpaths::to_openvr_paths(paths.into_iter().collect::<Vec<_>>().as_slice());

    openvrpaths::save_openvr_paths_json(&openvr_paths_json)
}

fn get_driver_dir_from_registered() -> StrResult<PathBuf> {
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
