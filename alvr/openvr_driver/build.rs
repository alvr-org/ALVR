use cbindgen::Language;
use regex::Regex;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let alvr_streamer_header_dir = Path::new("../../build");
    let alvr_streamer_header_path = alvr_streamer_header_dir.join("alvr_streamer.h");

    cbindgen::Builder::new()
        .with_crate("../server")
        .with_language(Language::C)
        .generate()
        .unwrap()
        .write_to_file(&alvr_streamer_header_path);

    fs::write(
        &alvr_streamer_header_path,
        format!(
            "#pragma once\n\n{}",
            fs::read_to_string(&alvr_streamer_header_path).unwrap()
        ),
    )
    .unwrap();

    let openvr_driver_header_string = fs::read_to_string("cpp/openvr_driver.h").unwrap();

    let property_finder =
        Regex::new(r"\t(Prop_[A-Za-z\d_]*_(?:Bool|Int32|Uint64|Float|String|Vector3))\W").unwrap();

    let mut mappings_fn_string: String = String::from(
        r#"#pragma once

#include <string.h>
#include "openvr_driver.h"

inline vr::ETrackedDeviceProperty tracked_device_property_name_to_key(const char *prop_name) {
"#,
    );

    // Note: this generates disjoint if branches. This is a workaround for MSVC nesting limit of 128
    for entry in property_finder.captures_iter(&openvr_driver_header_string) {
        mappings_fn_string.push_str(&format!(
            r#"    if (strcmp(prop_name, "{}") == 0) {{
        return vr::{};
    }}
"#,
            &entry[1], &entry[1],
        ));
    }

    mappings_fn_string.push_str(
        r#"
    return vr::Prop_Invalid;
}"#,
    );

    fs::write(
        "../../build/openvr_properties_mapping.h",
        mappings_fn_string,
    )
    .unwrap();

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .files([
            "cpp/paths.cpp",
            "cpp/hmd.cpp",
            "cpp/controller.cpp",
            "cpp/generic_tracker.cpp",
            "cpp/chaperone.cpp",
            "cpp/driver.cpp",
        ])
        .include("cpp")
        .include(alvr_streamer_header_dir)
        .compile("bindings");

    bindgen::builder()
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .header("cpp/bindings.h")
        .derive_default(true)
        .enable_cxx_namespaces()
        .prepend_enum_name(false)
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("bindings.rs"))
        .unwrap();

    let artifacts_dir = out_dir.join("../../..");

    #[cfg(windows)]
    fs::copy(
        artifacts_dir.join("alvr_server.dll.lib"),
        artifacts_dir.join("alvr_server.lib"),
    )
    .ok();

    println!(
        "cargo:rustc-link-search={}",
        artifacts_dir.to_string_lossy()
    );

    // Note: compilation problems when using static lib
    // todo: rename to alvr_streamer
    println!("cargo:rustc-link-lib=dylib=alvr_server");

    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("resources/lib")
            .to_string_lossy()
    );
    println!("cargo:rustc-link-lib=openvr_api");

    println!("cargo:rerun-if-changed=cpp");
}
