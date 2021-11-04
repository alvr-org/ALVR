use crate::dashboard::pretty::tabs::InitData;

use super::reset;
use iced::{slider, text_input};
use serde::de::DeserializeOwned;
use serde_json as json;
use settings_schema::NumericGuiType;
use std::ops::RangeInclusive;

struct SliderState<T> {
    state: slider::State,
    range: RangeInclusive<T>,
}

pub struct Control<T> {
    default: T,
    value: T,
    slider_state: Option<SliderState<T>>,
    textbox_state: text_input::State,
    reset_control: reset::Control,
}

impl<T: Copy + PartialEq + DeserializeOwned> Control<T> {
    pub fn new(
        data: InitData<(T, Option<T>, Option<T>, Option<T>, Option<NumericGuiType>)>,
    ) -> Self {
        let (default, min, max, step, gui) = data.schema;

        // let value = json::from_value(data.session).unwrap();

        Self {
            default,
            value: default,
            slider_state: None, // todo
            textbox_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }
}
