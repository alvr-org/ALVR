pub mod array;
pub mod boolean;
pub mod choice;
pub mod dictionary;
pub mod help;
pub mod notice;
pub mod number;
pub mod optional;
pub mod presets;
pub mod reset;
pub mod section;
pub mod switch;
pub mod text;
pub mod vector;

use alvr_packets::{PathSegment, PathValuePair};
use alvr_session::settings_schema::SchemaNode;
use eframe::egui::Ui;
use serde_json as json;

const INDENTATION_STEP: f32 = 20.0;

fn set_single_value(
    nesting_info: &NestingInfo,
    leaf: PathSegment,
    new_value: json::Value,
) -> Option<PathValuePair> {
    let mut path = nesting_info.path.clone();
    path.push(leaf);

    Some(PathValuePair {
        path,
        value: new_value,
    })
}

fn grid_flow_inline(ui: &mut Ui, allow_inline: bool) {
    if !allow_inline {
        // Note: ui.add_space() does not work
        ui.label(" ");
    }
}

fn grid_flow_block(ui: &mut Ui, allow_inline: bool) {
    if allow_inline {
        ui.end_row();
    }
}

#[derive(Clone)]
pub struct NestingInfo {
    pub path: Vec<PathSegment>,
    pub indentation_level: usize,
}

pub enum SettingControl {
    Section(section::Control),
    Choice(choice::Control),
    Optional(optional::Control),
    Switch(switch::Control),
    Boolean(boolean::Control),
    Text(text::Control),
    Numeric(number::Control),
    Array(array::Control),
    None,
}

impl SettingControl {
    pub fn new(nesting_info: NestingInfo, schema: SchemaNode) -> Self {
        match schema {
            SchemaNode::Section(entries) => {
                Self::Section(section::Control::new(nesting_info, entries))
            }
            SchemaNode::Choice {
                default,
                variants,
                gui,
            } => Self::Choice(choice::Control::new(nesting_info, default, variants, gui)),
            SchemaNode::Optional {
                default_set,
                content,
            } => Self::Optional(optional::Control::new(nesting_info, default_set, *content)),
            SchemaNode::Switch {
                default_enabled,
                content,
            } => Self::Switch(switch::Control::new(
                nesting_info,
                default_enabled,
                *content,
            )),
            SchemaNode::Boolean { default } => {
                Self::Boolean(boolean::Control::new(nesting_info, default))
            }
            SchemaNode::Number {
                default,
                ty,
                gui,
                suffix,
            } => Self::Numeric(number::Control::new(nesting_info, default, ty, gui, suffix)),
            SchemaNode::Text { default } => Self::Text(text::Control::new(nesting_info, default)),
            SchemaNode::Array(schema_array) => {
                Self::Array(array::Control::new(nesting_info, schema_array))
            }
            // SchemaNode::Vector { default_element, default } => todo!(),
            // SchemaNode::Dictionary { default_key, default_value, default } => todo!(),
            _ => Self::None,
        }
    }

    // inline: first field child, could be rendered beside the field label
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        match self {
            Self::Section(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Choice(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Optional(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Switch(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Boolean(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Text(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Numeric(control) => control.ui(ui, session_fragment, allow_inline),
            Self::Array(control) => control.ui(ui, session_fragment, allow_inline),
            Self::None => {
                grid_flow_inline(ui, allow_inline);
                ui.add_enabled_ui(false, |ui| ui.label("Unimplemented UI"));

                None
            }
        }
    }
}
