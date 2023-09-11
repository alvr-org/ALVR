use regex::Regex;
use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let openvr_driver_header_string =
        fs::read_to_string("../server/cpp/openvr/headers/openvr_driver.h").unwrap();

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
            let mut name = cap[1].replace('_', "");
            if code == "1007" {
                name = "HardwareRevisionString".into();
            } else if code == "1017" {
                name = "HardwareRevisionUint64".into();
            }
            PropInfo {
                name,
                ty: cap[2].into(),
                code,
            }
        })
        .collect::<Vec<_>>();

    let mut mappings_fn_string: String = String::from(
        r"#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
pub enum OpenvrProperty {",
    );

    for info in &prop_info {
        let ty = match info.ty.as_str() {
            "Bool" => "bool",
            "Int32" => "i32",
            "Uint64" => "u64",
            "Float" => "f32",
            "String" => "String",
            "Vector3" => "[f32; 3]",
            _ => "()",
        };

        write!(
            mappings_fn_string,
            r"
    {}({}) = {},",
            &info.name, ty, &info.code
        )
        .unwrap();
    }

    mappings_fn_string.push_str(
        r"
}

#[derive(Clone, Debug)]
pub enum OpenvrPropValue {
    Bool(bool),
    Float(f32),
    Int32(i32),
    Uint64(u64),
    Vector3([f32; 3]),
    Double(f64),
    String(String),
}

impl OpenvrProperty {
    pub fn into_key_value(self) -> (u32, OpenvrPropValue) {
        match self {",
    );

    for info in &prop_info {
        write!(
            mappings_fn_string,
            r"
            OpenvrProperty::{}(value) => ({}, OpenvrPropValue::{}(value)),",
            &info.name, info.code, info.ty,
        )
        .unwrap();
    }

    mappings_fn_string.push_str(
        r"
        }
    }
}

static OPENVR_PROPS_DEFAULT: alvr_common::once_cell::sync::Lazy<OpenvrPropertyDefault> = 
    alvr_common::once_cell::sync::Lazy::new(|| OpenvrPropertyDefault {",
    );

    for info in &prop_info {
        let default = match info.ty.as_str() {
            "Bool" => "false",
            "Int32" => "0",
            "Uint64" => "0",
            "Float" => "0.0",
            "String" => "String::new()",
            "Vector3" => {
                r"ArrayDefault {
            gui_collapsed: false,
            content: [0.0, 0.0, 0.0],
        }"
            }

            _ => "()",
        };

        write!(
            mappings_fn_string,
            r"
        {}: {},",
            &info.name, default
        )
        .unwrap();
    }

    mappings_fn_string.push_str(
        r"
        variant: OpenvrPropertyDefaultVariant::TrackingSystemName,
    });
",
    );

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("openvr_property_keys.rs"),
        mappings_fn_string,
    )
    .unwrap();
}
