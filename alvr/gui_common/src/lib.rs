mod basic_components;
pub mod theme;

pub use basic_components::*;

use std::{ops::Deref, sync::atomic::AtomicUsize};

pub fn get_id() -> usize {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone)]
pub struct DisplayString {
    pub id: String,
    pub display: String,
}

impl From<(String, String)> for DisplayString {
    fn from((id, display): (String, String)) -> Self {
        Self { id, display }
    }
}

impl Deref for DisplayString {
    type Target = String;

    fn deref(&self) -> &String {
        &self.id
    }
}
