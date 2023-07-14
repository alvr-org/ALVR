use crate::openvrpaths;
use alvr_common::{
    anyhow::{bail, Result},
    ToAny,
};
use serde_json as json;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

pub fn get_registered_drivers() -> Result<Vec<PathBuf>> {
    Ok(openvrpaths::from_openvr_paths(
        openvrpaths::load_openvr_paths_json()?
            .get_mut("external_drivers")
            .to_any()?,
    ))
}

pub fn driver_registration(driver_paths: &[PathBuf], register: bool) -> Result<()> {
    let mut openvr_paths_json = openvrpaths::load_openvr_paths_json()?;
    let paths_json_ref = openvr_paths_json.get_mut("external_drivers").to_any()?;

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

pub fn get_driver_dir_from_registered() -> Result<PathBuf> {
    for dir in get_registered_drivers()? {
        let maybe_driver_name = || -> Result<_> {
            let manifest_string = fs::read_to_string(dir.join("driver.vrdrivermanifest"))?;
            let mut manifest_map =
                json::from_str::<HashMap<String, json::Value>>(&manifest_string)?;

            manifest_map.remove("name").to_any()
        }();

        if let Ok(json::Value::String(str)) = maybe_driver_name {
            if str == "alvr_server" {
                return Ok(dir);
            }
        }
    }

    bail!("ALVR driver path not registered")
}
