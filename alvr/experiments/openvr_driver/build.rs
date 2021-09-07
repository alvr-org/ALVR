use regex::Regex;
use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .files(["cpp/tracked_devices.cpp", "cpp/driver.cpp"])
        .include("cpp")
        .compile("bindings");

    bindgen::builder()
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .clang_arg("-Icpp/bindgen_workaround")
        .header("cpp/bindings.h")
        .derive_default(true)
        .enable_cxx_namespaces()
        .prepend_enum_name(false)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

    let openvr_driver_header_string = fs::read_to_string("cpp/openvr_driver.h").unwrap();

    let property_finder = Regex::new(
        r"\t(Prop_[A-Za-z\d_]*_(?:Bool|Int32|Uint64|Float|String|Vector3))[ \t]*= (\d*),",
    )
    .unwrap();

    let mut mappings_fn_string: String = String::from(
        r"
        pub fn tracked_device_property_name_to_key(prop_name: &str) -> Result<vr::ETrackedDeviceProperty, String> {
            match prop_name {
        ",
    );

    for entry in property_finder.captures_iter(&openvr_driver_header_string) {
        mappings_fn_string.push_str(&format!(
            r#"
                "{}" => Ok({}),
            "#,
            &entry[1], &entry[2]
        ));
    }

    mappings_fn_string.push_str(
        r#"
                _ => Err(format!("{} property not found or not supported", prop_name)),
            }
        }
        "#,
    );

    fs::write(out_dir.join("properties_mappings.rs"), mappings_fn_string).unwrap();

    println!("cargo:rerun-if-changed=cpp");
}
