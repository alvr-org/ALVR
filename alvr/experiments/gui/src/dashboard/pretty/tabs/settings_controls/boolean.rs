use serde_json as json;

use super::{reset, InitData};

pub struct Control {
    default: bool,
    value: bool,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<bool>) -> Self {
        // let value = json::from_value(data.session).unwrap();

        Self {
            default: data.schema,
            value: false,
            reset_control: reset::Control::new(),
        }
    }
}
