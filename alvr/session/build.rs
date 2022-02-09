use regex::Regex;
use std::{env, fs, path::PathBuf};

fn main() {
    let openvr_driver_header_string =
        fs::read_to_string("../openvr_driver/cpp/openvr_driver.h").unwrap();

    let property_finder = Regex::new(
        r"\tProp_([A-Za-z\d_]+)_(?:Bool|Int32|Uint64|Float|String|Vector3)[\t ]+= ([0-9]+)",
    )
    .unwrap();

    let mut mappings_fn_string: String = String::from(
        r"#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum OpenvrPropertyKey {",
    );

    // Note: this generates disjoint if branches. This is a workaround for MSVC nesting limit of 128
    for entry in property_finder.captures_iter(&openvr_driver_header_string) {
        // exclude repeated property
        if &entry[1] != "HardwareRevision" {
            mappings_fn_string.push_str(&format!(
                r"
    {} = {},",
                &entry[1].replace('_', ""),
                &entry[2],
            ));
        }
    }

    mappings_fn_string.push_str(
        r"
}",
    );

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("openvr_property_keys.rs"),
        mappings_fn_string,
    )
    .unwrap();
}
