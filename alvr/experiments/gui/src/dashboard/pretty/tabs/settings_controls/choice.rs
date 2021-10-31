use std::collections::HashMap;

use serde_json as json;
use settings_schema::EntryData;

use crate::dashboard::RequestHandler;

use super::SettingEvent;

#[derive(Clone, Debug)]
pub enum ChoiceEvent {
    SettingsUpdated(json::Value),
    VariantClick(String),
    Inner(SettingEvent), // the variant is known,
}

pub struct ChoiceControl {}

impl ChoiceControl {
    pub fn new(
        path: String,
        default: String,
        variants: Vec<(String, Option<EntryData>)>,
        session: json::Value,
        request_handler: &mut RequestHandler,
    ) -> Self {
        let mut session_map = json::from_value::<HashMap<String, json::Value>>(session).unwrap();

        let variant = session_map
            .remove("variant")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();

        Self {}
    }
}
