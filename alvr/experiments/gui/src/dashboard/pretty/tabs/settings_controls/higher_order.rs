use crate::dashboard::pretty::InitData;

use super::{boolean, choice, reset};
use iced::button;

pub enum Control {
    Action {
        applied: bool,
        button_state: button::State,
    },
    Boolean {
        default: bool,
        value: Option<bool>,
        reset_control: reset::Control,
    },
    Choice {
        default: String,
        entries: Vec<String>,
        selection: Option<usize>,
        reset_control: reset::Control,
    },
}

impl Control {
    // todo: needs new settings-schema
    pub fn new(schema: ()) -> Self {
        Self::Action {
            applied: false,
            button_state: button::State::new(),
        }
    }
}
