use alvr_common::data::*;
use std::fs;

// Save schema and settings to disk for debug purposes

fn main() {
    let default = settings_default();

    fs::write(
        "../../build/session.json",
        serde_json::to_string_pretty(&default).unwrap(),
    )
    .unwrap();

    fs::write(
        "../../build/settings_schema.json",
        serde_json::to_string_pretty(&settings_schema(default)).unwrap(),
    )
    .unwrap();
}
