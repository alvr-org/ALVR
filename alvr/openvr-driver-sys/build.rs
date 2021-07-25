use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_dir_include_flag_string = format!("-I{}", out_path.to_string_lossy());

    let mut build = cc::Build::new();
    let mut build = build
        .cpp(true)
        .flag("-std=c++11")
        .file("src/bindings.cpp")
        .flag("-Isrc")
        .flag("-Iinclude")
        .flag(&out_dir_include_flag_string);
    if !cfg!(windows) {
        build = build.flag("-Wno-unused-parameter");
    }
    build.compile("bindings");

    bindgen::builder()
        .clang_arg("-xc++")
        .clang_arg("-std=c++11")
        .header("src/openvr_driver_capi.h")
        .clang_arg("-Isrc")
        .clang_arg("-Iinclude")
        .clang_arg(&out_dir_include_flag_string)
        .layout_tests(false)
        .enable_cxx_namespaces()
        .default_enum_style(bindgen::EnumVariation::Consts)
        .prepend_enum_name(false)
        .derive_default(true)
        // .rustified_enum("vr::ETrackedPropertyError")
        // .rustified_enum("vr::EHDCPError")
        // .rustified_enum("vr::EVRInputError")
        // .rustified_enum("vr::EVRSpatialAnchorError")
        // .rustified_enum("vr::EVRSettingsError")
        // .rustified_enum("vr::EIOBufferError")
        .generate_inline_functions(true)
        .blocklist_function("vr::.*")
        .blocklist_type("vr::IVRSettings")
        .blocklist_type("vr::CVRSettingHelper")
        .blocklist_type("vr::ITrackedDeviceServerDriver")
        .blocklist_type("vr::IVRDisplayComponent")
        .blocklist_type("vr::IVRDriverDirectModeComponent")
        .opaque_type("vr::ICameraVideoSinkCallback")
        .blocklist_type("vr::IVRCameraComponent")
        .opaque_type("vr::IVRDriverContext")
        .blocklist_type("vr::IServerTrackedDeviceProvider")
        .blocklist_type("vr::IVRWatchdogProvider")
        .blocklist_type("vr::IVRCompositorPluginProvider")
        .blocklist_type("vr::IVRProperties")
        .blocklist_type("vr::CVRPropertyHelpers")
        .blocklist_type("vr::IVRDriverInput")
        .blocklist_type("vr::IVRDriverLog")
        .blocklist_type("vr::IVRServerDriverHost")
        .blocklist_type("vr::IVRCompositorDriverHost")
        .blocklist_type("vr::CVRHiddenAreaHelpers")
        .blocklist_type("vr::IVRWatchdogHost")
        .blocklist_type("vr::IVRVirtualDisplay")
        .blocklist_type("vr::IVRResources")
        .blocklist_type("vr::IVRIOBuffer")
        .blocklist_type("vr::IVRDriverManager")
        .blocklist_type("vr::IVRDriverSpatialAnchors")
        .blocklist_type("vr::COpenVRDriverContext")
        .generate()
        .expect("bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("bindings.rs");

    let openvr_driver_header_path_str = Path::new("include/openvr_driver.h");
    let openvr_driver_header_string =
        fs::read_to_string(openvr_driver_header_path_str).expect("openvr header not found");

    let property_finder = regex::Regex::new(
        r"\t(Prop_[A-Za-z\d_]*_(?:Bool|Int32|Uint64|Float|String|Vector3))[ \t]*= (\d*),",
    )
    .unwrap();

    let mut mappings_fn_string: String = String::from(
        r"
        pub fn tracked_device_property_name_to_key(prop_name: &str) -> Result<ETrackedDeviceProperty, String> {
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

    fs::write(out_path.join("properties_mappings.rs"), mappings_fn_string)
        .expect("Cannot write properties_mappings.rs");
}
