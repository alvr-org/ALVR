use super::{HigherOrderControl, SettingControl, ShowMode};
use crate::dashboard::{
    pretty::tabs::{
        ArrayControl, BooleanControl, ChoiceControl, DictionaryControl, FloatControl,
        IntegerControl, SettingsPanelEvent, SwitchControl, TextControl, VectorControl,
    },
    RequestHandler,
};
use iced::{Element, Text};
use serde_json as json;
use settings_schema::{EntryData, SchemaNode};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum SectionEvent {
    SettingsUpdated(json::Value),
    Inner {
        entry: String,
        event: SettingsPanelEvent,
    },
}

struct SectionEntry {
    show_mode: ShowMode,
    display_name: String,
    control: SettingControl,
}

pub struct SectionControl {
    entries: Vec<SectionEntry>,
}

impl SectionControl {
    pub fn new(
        path: String,
        schema: Vec<(String, Option<EntryData>)>,
        session: json::Value,
        request_handler: &mut RequestHandler,
    ) -> Self {
        let session_entries = json::from_value::<HashMap<String, json::Value>>(session).unwrap();

        let entries = schema
            .into_iter()
            .map(|(name, maybe_data)| {
                if let Some(data) = maybe_data {
                    let show_mode = if data.advanced {
                        ShowMode::Advanced
                    } else {
                        ShowMode::Always
                    };
                    let path = format!("{}.{}", path, name);
                    let session = session_entries.get(&name).unwrap().clone();

                    SectionEntry {
                        show_mode,
                        display_name: name,
                        control: SettingControl::new(path, data.content, session, request_handler),
                    }
                } else {
                    // todo
                    SectionEntry {
                        show_mode: ShowMode::Basic,
                        display_name: name,
                        control: SettingControl::HigherOrder(HigherOrderControl {}),
                    }
                }
            })
            .collect();

        Self { entries }
    }

    pub fn update(&mut self, event: SectionEvent, request_handler: &mut RequestHandler) {}

    pub fn label_views(&mut self, advanced: bool) -> Vec<Element<SectionEvent>> {
        vec![Text::new("unimplemented").into()]
    }

    pub fn control_views(&mut self, advanced: bool) -> Vec<Element<SectionEvent>> {
        vec![Text::new("unimplemented").into()]
    }
}
