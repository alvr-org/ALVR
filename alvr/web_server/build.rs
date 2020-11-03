use alvr_common::data::*;

fn main() {
    println!(
        "cargo:rustc-env=SETTINGS_SCHEMA={}",
        serde_json::to_string(&settings_schema(session_settings_default())).unwrap()
    );
}
