mod array;
mod audio_dropdown;
mod boolean;
mod choice;
mod dictionary;
mod help;
mod higher_order;
mod notice;
mod numeric;
mod optional;
mod reset;
mod section;
mod switch;
mod text;
mod vector;

use std::collections::HashMap;

pub use array::*;
pub use audio_dropdown::*;
pub use boolean::*;
pub use choice::*;
pub use dictionary::*;
pub use help::*;
pub use higher_order::*;
pub use notice::*;
pub use numeric::*;
pub use optional::*;
pub use reset::*;
pub use section::*;
pub use switch::*;
pub use text::*;
pub use vector::*;

use crate::dashboard::RequestHandler;
use iced::Element;
use serde_json as json;
use settings_schema::SchemaNode;

enum ShowMode {
    Basic,
    Advanced,
    Always,
}

#[derive(Clone, Debug)]
pub enum SettingEvent {
    SettingsUpdated(json::Value),
    Section(Box<SectionEvent>),
    Choice(Box<ChoiceEvent>),
}

pub enum SettingControl {
    Section(Box<SectionControl>),
    Choice(Box<ChoiceControl>),
    Optional(Box<OptionalControl>),
    Switch(Box<SwitchControl>),
    Boolean(BooleanControl),
    Integer(IntegerControl),
    Float(FloatControl),
    Text(TextControl),
    Array(Box<ArrayControl>),
    Vector(Box<VectorControl>),
    Dictionary(Box<DictionaryControl>),
    HigherOrder(HigherOrderControl),
    AudioDropdown(AudioDropdownControl),
}

impl SettingControl {
    pub fn new(
        path: String,
        schema: SchemaNode,
        session: json::Value,
        request_handler: &mut RequestHandler,
    ) -> Self {
        match schema {
            SchemaNode::Section { entries } => SettingControl::Section(Box::new(
                SectionControl::new(path, entries, session, request_handler),
            )),
            SchemaNode::Choice { default, variants } => SettingControl::Choice(Box::new(
                ChoiceControl::new(path, default, variants, session, request_handler),
            )),
            SchemaNode::Optional {
                default_set,
                content,
            } => SettingControl::Optional(Box::new(OptionalControl {})),
            SchemaNode::Switch {
                default_enabled,
                content_advanced,
                content,
            } => SettingControl::Switch(Box::new(SwitchControl {})),
            SchemaNode::Boolean { default } => {
                SettingControl::Boolean(BooleanControl::new(path, default, session))
            }
            SchemaNode::Integer {
                default,
                min,
                max,
                step,
                gui,
            } => SettingControl::Integer(IntegerControl::new(
                path, default, min, max, step, gui, session,
            )),
            SchemaNode::Float {
                default,
                min,
                max,
                step,
                gui,
            } => SettingControl::Float(FloatControl::new(
                path, default, min, max, step, gui, session,
            )),
            SchemaNode::Text { default } => SettingControl::Text(TextControl {}),
            SchemaNode::Array(_) => SettingControl::Array(Box::new(ArrayControl {})),
            SchemaNode::Vector {
                default_element,
                default,
            } => SettingControl::Vector(Box::new(VectorControl {})),
            SchemaNode::Dictionary {
                default_key,
                default_value,
                default,
            } => SettingControl::Dictionary(Box::new(DictionaryControl {})),
        }
    }

    pub fn update(&mut self, event: SettingEvent, request_handler: &mut RequestHandler) {
        match (self, event) {
            (SettingControl::Section(control), SettingEvent::Section(event)) => {
                control.update(*event, request_handler)
            }
            _ => unreachable!(),
        }
    }

    // List of labels or left-side controls. If no controls are required, spaces must be inserted
    pub fn label_elements(&mut self, advanced: bool) -> Vec<Element<SettingEvent>> {
        todo!()
    }

    // List of right-side controls. The first one is to the right of the entry label
    pub fn control_elements(&mut self, advanced: bool) -> Vec<Element<SettingEvent>> {
        todo!()
    }
}
