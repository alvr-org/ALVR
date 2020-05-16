use alvr_common::data::*;

fn main() {
    println!(
        "cargo:rustc-env=SETTINGS_SCHEMA={}",
        serde_json::to_string(&settings_schema(settings_cache_default())).unwrap()
    );
}
