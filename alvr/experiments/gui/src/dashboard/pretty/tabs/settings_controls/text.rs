use super::{
    reset, DrawingData, DrawingResult, InitData, SettingControlEvent, SettingControlEventType,
    UpdatingData, ROW_HEIGHT,
};
use crate::dashboard::pretty::theme::{TextInputStyle, TooltipStyle};
use iced::{text_input, tooltip::Position, Alignment, Length, Row, Space, TextInput, Tooltip};
use serde_json as json;

pub struct Control {
    default: String,
    value: String,
    control_state: text_input::State,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<String>) -> Self {
        Self {
            default: data.schema,
            value: "".into(),
            control_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, data: UpdatingData) {
        match data.event {
            SettingControlEventType::SessionUpdated(session) => {
                self.value = json::from_value(session).unwrap();
                self.reset_control.update(self.value != self.default);
            }
            SettingControlEventType::TempValueChanged(value) => self.value = value,
            event => {
                let value = if event == SettingControlEventType::ApplyValue {
                    self.value.clone()
                } else {
                    self.default.clone()
                };

                (data.request_handler)(format!(
                    r#"
                        let session = load_session();
                        {} = "{value}";
                        store_session(session);
                    "#,
                    data.string_path,
                ))
                .unwrap();
            }
        }
    }

    pub fn view(&mut self, _: &DrawingData) -> DrawingResult {
        let inline = Row::new()
            .push(
                Row::new()
                    .push(
                        Tooltip::new(
                            TextInput::new(&mut self.control_state, "", &self.value, |s| {
                                SettingControlEvent {
                                    path: vec![],
                                    event_type: SettingControlEventType::TempValueChanged(s),
                                }
                            })
                            .padding([0, 5])
                            .style(TextInputStyle)
                            .on_submit(SettingControlEvent {
                                path: vec![],
                                event_type: SettingControlEventType::ApplyValue,
                            }),
                            "Press Enter to apply the value",
                            Position::Bottom,
                        )
                        .style(TooltipStyle),
                    )
                    .padding([5, 0])
                    .width(Length::Fill),
            )
            .push(self.reset_control.view())
            .height(ROW_HEIGHT)
            .spacing(5)
            .align_items(Alignment::Center);

        DrawingResult {
            inline: Some(inline.into()),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
