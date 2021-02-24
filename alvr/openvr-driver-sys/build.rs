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
        .blacklist_function("vr::.*")
        .blacklist_type("vr::IVRSettings")
        .blacklist_type("vr::CVRSettingHelper")
        .blacklist_type("vr::ITrackedDeviceServerDriver")
        .blacklist_type("vr::IVRDisplayComponent")
        .blacklist_type("vr::IVRDriverDirectModeComponent")
        .opaque_type("vr::ICameraVideoSinkCallback")
        .blacklist_type("vr::IVRCameraComponent")
        .opaque_type("vr::IVRDriverContext")
        .blacklist_type("vr::IServerTrackedDeviceProvider")
        .blacklist_type("vr::IVRWatchdogProvider")
        .blacklist_type("vr::IVRCompositorPluginProvider")
        .blacklist_type("vr::IVRProperties")
        .blacklist_type("vr::CVRPropertyHelpers")
        .blacklist_type("vr::IVRDriverInput")
        .blacklist_type("vr::IVRDriverLog")
        .blacklist_type("vr::IVRServerDriverHost")
        .blacklist_type("vr::IVRCompositorDriverHost")
        .blacklist_type("vr::CVRHiddenAreaHelpers")
        .blacklist_type("vr::IVRWatchdogHost")
        .blacklist_type("vr::IVRVirtualDisplay")
        .blacklist_type("vr::IVRResources")
        .blacklist_type("vr::IVRIOBuffer")
        .blacklist_type("vr::IVRDriverManager")
        .blacklist_type("vr::IVRDriverSpatialAnchors")
        .blacklist_type("vr::COpenVRDriverContext")
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
