use super::{reset, DrawingData, DrawingResult, ROW_HEIGHT};
use iced::{button, Space, Text};

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

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        DrawingResult {
            inline: Some(Text::new("unimplemented").height(ROW_HEIGHT).into()),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
