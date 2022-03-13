use crate::PathSegment;

use super::{
    reset, DrawingData, DrawingResult, InitData, SettingControl, SettingControlEvent,
    SettingControlEventType, UpdatingData, ROW_HEIGHT,
};
use iced::{Alignment, Length, Row, Space, Toggler};
use serde_json as json;
use settings_schema::{SchemaNode, SwitchDefault};

pub struct Control {
    default_enabled: bool,
    content_advanced: bool,
    enabled: bool,
    inner_control: SettingControl,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<(bool, bool, Box<SchemaNode>)>) -> Self {
        let (default_enabled, content_advanced, content_schema) = data.schema;

        Self {
            default_enabled,
            content_advanced,
            enabled: false,
            inner_control: SettingControl::new(InitData {
                schema: *content_schema,
                trans: (),
            }),
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, mut data: UpdatingData) {
        if let SettingControlEventType::SessionUpdated(session) = data.event {
            let session_switch = json::from_value::<SwitchDefault<json::Value>>(session).unwrap();

            self.enabled = session_switch.enabled;
            self.reset_control
                .update(session_switch.enabled != self.default_enabled);

            self.inner_control.update(UpdatingData {
                event: SettingControlEventType::SessionUpdated(session_switch.content),
                ..data
            });
        } else if data.index_path.pop().is_some() {
            data.segment_path.push(PathSegment::Name("content".into()));
            self.inner_control.update(UpdatingData {
                segment_path: data.segment_path,
                ..data
            })
        } else {
            self.enabled = if data.event == SettingControlEventType::Toggle {
                !self.enabled
            } else {
                self.default_enabled
            };

            data.segment_path.push(PathSegment::Name("enabled".into()));
            data.data_interface
                .set_single_value(data.segment_path, &self.enabled.to_string());
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let inline = Row::new()
            .push(Toggler::new(self.enabled, None, |_| SettingControlEvent {
                path: vec![],
                event_type: SettingControlEventType::Toggle,
            }))
            .push(Space::with_width(Length::Fill))
            .push(self.reset_control.view())
            .height(ROW_HEIGHT)
            .align_items(Alignment::Center)
            .into();

        let (left, right) = if self.enabled && (data.advanced || !self.content_advanced) {
            super::draw_result(self.inner_control.view(data), 0)
        } else {
            (
                Space::with_height(0.into()).into(),
                Space::with_height(0.into()).into(),
            )
        };

        DrawingResult {
            inline: Some(inline),
            left,
            right,
        }
    }
}
