use serde::{Deserialize, Serialize};
use serde_json as json;
use settings_schema::ChoiceControlType;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone)]
pub enum PresetModifierOperation {
    Assign(json::Value),
}

#[derive(Serialize, Deserialize)]
pub struct PresetModifier {
    // session-style path
    pub target_path: String,
    pub operation: PresetModifierOperation,
}

#[derive(Serialize, Deserialize)]
pub struct HigherOrderChoiceOption {
    pub display_name: String,
    pub modifiers: Vec<PresetModifier>,
    pub content: Option<PresetSchemaNode>,
}

#[derive(Serialize, Deserialize)]
pub struct HigherOrderChoiceSchema {
    pub name: String,
    pub strings: HashMap<String, String>,
    pub flags: HashSet<String>,
    pub options: Vec<HigherOrderChoiceOption>,
    pub default_option_index: usize,
    pub gui: ChoiceControlType,
}

#[derive(Serialize, Deserialize)]
pub enum PresetSchemaNode {
    HigherOrderChoice(HigherOrderChoiceSchema),

    // session-style path
    Mirror(String),
}
