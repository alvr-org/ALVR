use crate::PathSegment;

use super::{
    DrawingData, DrawingResult, InitData, SettingControl, SettingControlEventType, UpdatingData,
};
use iced::Column;
use serde_json as json;
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

    pub fn update(&mut self, mut data: UpdatingData) {
        if let SettingControlEventType::SessionUpdated(session) = data.event {
            let session_entries = json::from_value::<Vec<json::Value>>(session).unwrap();

            for (index, entry) in self.entries.iter_mut().enumerate() {
                let session = session_entries[index].clone();
                entry.update(UpdatingData {
                    index_path: vec![],
                    segment_path: vec![],
                    event: SettingControlEventType::SessionUpdated(session),
                    data_interface: data.data_interface,
                })
            }
        } else {
            let index = data.index_path.pop().unwrap();
            data.segment_path.push(PathSegment::Index(index));

            let entry = &mut self.entries[index];
            entry.update(UpdatingData {
                segment_path: data.segment_path,
                ..data
            })
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let (left_controls, right_controls) = self
            .entries
            .iter_mut()
            .enumerate()
            .map(|(index, entry)| super::draw_result(entry.view(data), index))
            .unzip();

        DrawingResult {
            inline: None,
            left: Column::with_children(left_controls).into(),
            right: Column::with_children(right_controls).into(),
        }
    }
}
