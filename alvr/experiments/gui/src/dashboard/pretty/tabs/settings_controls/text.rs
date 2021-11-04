use super::{reset, InitData};
use iced::text_input;
use serde_json as json;

pub struct Control {
    default: String,
    value: String,
    control_state: text_input::State,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<String>) -> Self {
        // let value = json::from_value(data.session).unwrap();

        Self {
            default: data.schema,
            value: "".into(),
            control_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }
}
