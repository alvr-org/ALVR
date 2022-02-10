use super::{
    DrawingData, DrawingResult, InitData, SettingControl, SettingControlEvent,
    SettingControlEventType, UpdatingData,
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
                    path: vec![],
                    event: SettingControlEventType::SessionUpdated(session),
                    request_handler: data.request_handler,
                    string_path: String::new(),
                })
            }
        } else {
            let index = data.path.pop().unwrap();
            let entry = &mut self.entries[index];
            entry.update(UpdatingData {
                string_path: format!("{}[{index}]", data.string_path),
                ..data
            })
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let (left_controls, right_controls) = self
            .entries
            .iter_mut()
            .enumerate()
            .map(|(index, entry)| {
                let (left, right) = super::draw_result(entry.view(data));
                (
                    left.map(move |mut e: SettingControlEvent| {
                        e.path.push(index);
                        e
                    }),
                    right.map(move |mut e: SettingControlEvent| {
                        e.path.push(index);
                        e
                    }),
                )
            })
            .unzip();

        DrawingResult {
            inline: None,
            left: Column::with_children(left_controls).into(),
            right: Column::with_children(right_controls).into(),
        }
    }
}
