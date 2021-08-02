use std::ops::Deref;

pub mod dashboard;
pub mod theme;
pub mod translation;

#[derive(Clone)]
pub struct LocalizedId {
    pub id: String,
    pub trans: String,
}

impl Deref for LocalizedId {
    type Target = String;

    fn deref(&self) -> &String {
        &self.id
    }
}
