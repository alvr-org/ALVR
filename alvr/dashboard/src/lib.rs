use std::ops::Deref;

pub mod dashboard;
pub mod theme;

#[derive(Clone)]
pub struct DisplayString {
    pub id: String,
    pub display: String,
}

impl Deref for DisplayString {
    type Target = String;

    fn deref(&self) -> &String {
        &self.id
    }
}
