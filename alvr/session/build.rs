use regex::Regex;
use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let openvr_driver_header_string =
        fs::read_to_string(alvr_filesystem::workspace_dir().join("openvr/headers/openvr_driver.h"))
            .expect("Missing openvr header files, did you clone the submodule?\n");

    let property_finder = Regex::new(
        r"\tProp_([A-Za-z\d_]+)_(Bool|Int32|Uint64|Float|String|Vector3)[\t ]+= ([0-9]+)",
    )
    .unwrap();

    struct PropInfo {
        name: String,
        ty: String,
        code: String,
    }

    let prop_info = property_finder
        .captures_iter(&openvr_driver_header_string)
        .map(|cap| {
            let code = cap[3].into();
            let name = format!("{}{}", cap[1].replace('_', ""), &cap[2]);

            PropInfo {
                name,
                ty: cap[2].into(),
                code,
            }
        })
        .collect::<Vec<_>>();

    let mut mappings_fn_string: String = String::from(
        r"#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy, Debug)]
pub enum OpenvrPropKey {",
    );

    for info in &prop_info {
        write!(
            mappings_fn_string,
            r"
    {} = {},",
            &info.name, &info.code
        )
        .unwrap();
    }

    mappings_fn_string.push_str(
        r"
}

pub fn openvr_prop_key_to_type(key: OpenvrPropKey) -> OpenvrPropType {
    match key {",
    );

    for info in &prop_info {
        write!(
            mappings_fn_string,
            r"
        OpenvrPropKey::{} => OpenvrPropType::{},",
            &info.name, info.ty
        )
        .unwrap();
    }

    mappings_fn_string.push_str(
        r"
    }
}
",
    );

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("openvr_property_keys.rs"),
        mappings_fn_string,
    )
    .unwrap();
}
