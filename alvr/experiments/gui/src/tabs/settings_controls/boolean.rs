use super::{
    reset, DrawingData, DrawingResult, InitData, SettingControlEvent, SettingControlEventType,
    UpdatingData, ROW_HEIGHT,
};
use iced::{Alignment, Length, Row, Space, Toggler};
use serde_json as json;

pub struct Control {
    default: bool,
    value: bool,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<bool>) -> Self {
        Self {
            default: data.schema,
            value: false,
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, data: UpdatingData) {
        if let SettingControlEventType::SessionUpdated(session) = data.event {
            self.value = json::from_value(session).unwrap();
            self.reset_control.update(self.value != self.default);
        } else {
            if data.event == SettingControlEventType::Toggle {
                self.value = !self.value;
            } else if data.event == SettingControlEventType::ResetClick {
                self.value = self.default;
            }

            data.data_interface
                .set_single_value(data.segment_path, &self.value.to_string());
        }
    }

    pub fn view(&mut self, _: &DrawingData) -> DrawingResult {
        let inline = Row::new()
            .push(Toggler::new(self.value, None, |_| SettingControlEvent {
                path: vec![0],
                event_type: SettingControlEventType::Toggle,
            }))
            .push(Space::with_width(Length::Fill))
            .push(self.reset_control.view())
            .height(ROW_HEIGHT)
            .align_items(Alignment::Center)
            .into();

        DrawingResult {
            inline: Some(inline),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
