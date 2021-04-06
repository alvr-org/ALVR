mod dashboard;
mod logging_backend;

use dashboard::Dashboard;
use wasm_bindgen::prelude::*;
use yew::App;

use alvr_common::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    logging_backend::init();

    info!("Hello world");

    App::<Dashboard>::new().mount_to_body();
}
