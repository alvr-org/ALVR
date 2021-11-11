use iced::{Alignment, Length, Row, Space, Toggler};
use serde_json as json;

use crate::dashboard::pretty::tabs::{
    settings_controls::ROW_HEIGHT, SettingControlEvent, SettingControlEventType,
};

use super::{reset, DrawingData, DrawingResult, InitData, UpdatingData};

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
            let value = if data.event == SettingControlEventType::Toggle {
                !self.value
            } else {
                self.default
            };

            (data.request_handler)(format!(
                r#"
                    let session = load_session();
                    {} = {};
                    store_session(session);
                "#,
                data.string_path, value
            ));

            self.value = value;
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
