use alvr_packets::PathValuePair;
use serde::{Deserialize, Serialize};
use settings_schema::ChoiceControlType;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize)]
pub struct HigherOrderChoiceOption {
    pub name: String,
    pub modifiers: Vec<PathValuePair>,
    pub content: Option<PresetSchemaNode>,
}

#[derive(Serialize, Deserialize)]
pub struct HigherOrderChoiceSchema {
    pub name: String,
    pub strings: HashMap<String, String>,
    pub flags: HashSet<String>,
    pub options: Vec<HigherOrderChoiceOption>,
    pub default_option_name: String,
    pub gui: ChoiceControlType,
}

#[derive(Serialize, Deserialize)]
pub enum PresetSchemaNode {
    HigherOrderChoice(HigherOrderChoiceSchema),

    // session-style path
    Mirror(String),
}
