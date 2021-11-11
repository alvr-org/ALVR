use super::{draw_result, DrawingData, DrawingResult, InitData, SettingControl};
use iced::{Column, Container, Space, Text};
use settings_schema::SchemaNode;

pub struct Control {
    entries: Vec<SettingControl>,
}

impl Control {
    pub fn new(data: InitData<Vec<SchemaNode>>) -> Self {
        Self {
            entries: data
                .schema
                .into_iter()
                .map(|schema| SettingControl::new(InitData { schema, trans: () }))
                .collect(),
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let (left_controls, right_controls) = self
            .entries
            .iter_mut()
            .map(|entry| draw_result(entry.view(data)))
            .unzip();

        DrawingResult {
            inline: None,
            left: Column::with_children(left_controls).into(),
            right: Column::with_children(right_controls).into(),
        }
    }
}
