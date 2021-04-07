mod basic_components;
mod dashboard;
mod logging_backend;

use alvr_common::prelude::*;
use dashboard::Dashboard;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    logging_backend::init();

    info!("Hello world");

    yew::start_app::<Dashboard>();
}
