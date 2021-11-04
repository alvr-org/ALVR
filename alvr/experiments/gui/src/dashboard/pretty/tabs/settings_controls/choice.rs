use super::{reset, DrawingData, InitData, SettingControl};
use crate::dashboard::RequestHandler;
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

struct Variant {
    display_name: String,
    control: Option<SettingControl>,
}

pub struct Control {
    default: String,
    variants: Vec<Variant>,
    selection: usize,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<(String, Vec<(String, Option<EntryData>)>)>) -> Self {
        let (default, variants) = data.schema;

        Self {
            default,
            variants: vec![],
            selection: 0,
            reset_control: reset::Control::new(),
        }
    }
}

// pub fn new(
//     path: String,
//     default: String,
//     variants: Vec<(String, Option<EntryData>)>,
//     session: json::Value,
//     request_handler: &mut RequestHandler,
// ) -> Self {
//     let mut session_map = json::from_value::<HashMap<String, json::Value>>(session).unwrap();

//     let variant = session_map
//         .remove("variant")
//         .unwrap()
//         .as_str()
//         .unwrap()
//         .to_owned();

//     Self {}
// }
