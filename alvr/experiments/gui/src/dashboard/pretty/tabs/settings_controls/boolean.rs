use iced::{Space, Text, Toggler};
use serde_json as json;

use crate::dashboard::pretty::tabs::{
    settings_controls::ROW_HEIGHT_UNITS, SettingControlEvent, SettingControlEventType,
};

use super::{reset, DrawingData, DrawingResult, InitData, UpdatingData};

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

    pub fn update(&mut self, mut data: UpdatingData) {
        if let SettingControlEventType::SessionUpdated(session) = data.event {
            self.value = json::from_value(session).unwrap()
        } else {
            let value = if data.event == SettingControlEventType::Toggle {
                !self.value
            } else {
                self.default
            };

            println!(
                "{}",
                (data.request_handler)(format!(
                    r#"
                        let session = load_session();
                        {} = {};
                        store_session(session);
                    "#,
                    data.string_path, value
                ))
            );

            self.value = value;
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let inline = Toggler::new(self.value, None, |_| SettingControlEvent {
            path: vec![0],
            event_type: SettingControlEventType::Toggle,
        })
        .size(ROW_HEIGHT_UNITS)
        .into();

        DrawingResult {
            inline: Some(inline),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
