use serde_json as json;

pub struct BooleanControl {}

impl BooleanControl {
    pub fn new(path: String, default: bool, session: json::Value) -> Self {
        Self {}
    }
}
