// during development
#![allow(dead_code)]

mod basic_components;
mod dashboard;
mod logging_backend;

use alvr_common::prelude::*;
use dashboard::Dashboard;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;

static ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub fn get_id() -> String {
    format!("alvr{}", ID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[wasm_bindgen(start)]
pub fn main() {
    logging_backend::init();

    info!("Hello world");

    yew::start_app::<Dashboard>();
}
