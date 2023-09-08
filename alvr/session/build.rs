use regex::Regex;
use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let openvr_driver_header_string =
        fs::read_to_string("../server/cpp/openvr/headers/openvr_driver.h").unwrap();

    let property_finder = Regex::new(
        r"\tProp_([A-Za-z\d_]+)_(?:Bool|Int32|Uint64|Float|String|Vector3)[\t ]+= ([0-9]+)",
    )
    .unwrap();

    let mut mappings_fn_string: String = String::from(
        r"#[repr(u32)]
 #[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy, Debug)]
 pub enum OpenvrPropertyKey {",
    );

    for entry in property_finder.captures_iter(&openvr_driver_header_string) {
        // exclude repeated property
        if &entry[1] != "HardwareRevision" {
            write!(
                mappings_fn_string,
                r"
            {} = {},",
                &entry[1].replace('_', ""),
                &entry[2]
            )
            .unwrap();
        }
    }

    // Fix duplicated property
    mappings_fn_string.push_str(
        r"
    HardwareRevisionString = 1007,
    HardwareRevisionUint64 = 1017,
}",
    );

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("openvr_property_keys.rs"),
        mappings_fn_string,
    )
    .unwrap();
}
