use super::{
    reset, DrawingData, DrawingResult, InitData, SettingControl, SettingControlEvent,
    SettingControlEventType, UpdatingData, ROW_HEIGHT,
};
use crate::dashboard::pretty::theme::ButtonStyle;
use iced::{button, Alignment, Button, Length, Row, Space, Text};
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

struct VariantLabel {
    name: String,
    display_name: String,
    button_state: button::State,
}

struct VariantControl {
    name: String,
    advanced: bool,
    control: SettingControl,
}

pub struct Control {
    default: String,
    variant_indices: HashMap<String, usize>,
    variant_buttons: Vec<VariantLabel>,
    content_controls: Vec<Option<VariantControl>>,
    selection: usize,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<(String, Vec<(String, Option<EntryData>)>)>) -> Self {
        let (default, variants) = data.schema;

        let variant_indices = variants
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), index))
            .collect();

        let variant_buttons = variants
            .iter()
            .map(|(name, _)| {
                VariantLabel {
                    name: name.clone(),
                    display_name: name.clone(), // todo
                    button_state: button::State::new(),
                }
            })
            .collect();

        let content_controls = variants
            .into_iter()
            .map(|(name, maybe_data)| {
                maybe_data.map(|data| VariantControl {
                    name,
                    advanced: data.advanced,
                    control: SettingControl::new(InitData {
                        schema: data.content,
                        trans: (),
                    }),
                })
            })
            .collect();

        Self {
            default,
            variant_indices,
            variant_buttons,
            content_controls,
            selection: 0,
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, mut data: UpdatingData) {
        if let SettingControlEventType::SessionUpdated(session) = data.event {
            let mut session_variants =
                json::from_value::<HashMap<String, json::Value>>(session).unwrap();
            let variant_json = session_variants.remove("variant").unwrap();
            let variant = variant_json.as_str().unwrap();

            self.selection = self.variant_indices[variant];
            self.reset_control
                .update(self.selection != self.variant_indices[&self.default]);

            for content in self.content_controls.iter_mut().flatten() {
                let session_content = session_variants.remove(&content.name).unwrap();
                content.control.update(UpdatingData {
                    path: vec![],
                    event: SettingControlEventType::SessionUpdated(session_content),
                    request_handler: data.request_handler,
                    string_path: String::new(),
                })
            }
        } else if data.path.pop().is_some() {
            let selected_content = self.content_controls[self.selection].as_mut().unwrap();
            selected_content.control.update(UpdatingData {
                string_path: format!("{}.{}", data.string_path, selected_content.name),
                ..data
            })
        } else {
            let variant = if let SettingControlEventType::VariantClick(index) = data.event {
                &self.variant_buttons[index].name
            } else {
                &self.default
            };

            (data.request_handler)(format!(
                r#"
                    let session = load_session();
                    {}.variant = "{}";
                    store_session(session);
                "#,
                data.string_path, variant
            ))
            .unwrap();

            self.selection = self.variant_indices[variant];
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let buttons = self
            .variant_buttons
            .iter_mut()
            .enumerate()
            .map(|(index, variant)| {
                Button::new(&mut variant.button_state, Text::new(&variant.display_name))
                    .style(if index == self.selection {
                        ButtonStyle::Primary
                    } else {
                        ButtonStyle::Secondary
                    })
                    .on_press(SettingControlEvent {
                        path: vec![],
                        event_type: SettingControlEventType::VariantClick(index),
                    })
                    .into()
            })
            .collect();

        let inline = Row::with_children(buttons)
            .push(Space::with_width(Length::Fill))
            .push(self.reset_control.view())
            .height(ROW_HEIGHT)
            .align_items(Alignment::Center)
            .into();

        let maybe_block = if let Some(variant) = &mut self.content_controls[self.selection] {
            (data.advanced || !variant.advanced)
                .then(|| super::draw_result(variant.control.view(data)))
        } else {
            None
        };

        let (left, right) = maybe_block.unwrap_or_else(|| {
            (
                Space::with_height(0.into()).into(),
                Space::with_height(0.into()).into(),
            )
        });

        DrawingResult {
            inline: Some(inline),
            left,
            right,
        }
    }
}
