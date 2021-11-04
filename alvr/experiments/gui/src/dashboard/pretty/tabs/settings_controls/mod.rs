pub mod array;
pub mod audio_dropdown;
pub mod boolean;
pub mod choice;
pub mod dictionary;
pub mod higher_order;
pub mod numeric;
pub mod optional;
pub mod reset;
pub mod section;
pub mod switch;
pub mod text;
pub mod vector;

use crate::dashboard::RequestHandler;
use iced::Element;
use serde_json as json;
use settings_schema::SchemaNode;
use std::collections::HashMap;

enum ShowMode {
    Basic,
    Advanced,
    Always,
}

#[derive(Clone, Debug)]
pub enum SettingControlEventType {
    SessionUpdated(json::Value),
    ResetClick,
    Click,                         // For HOS/Action
    VariantClick(String),          // For Choice, HOS/Choice
    Toggle,                        // For Optional, Switch, Boolean, HOS/Boolean
    IntegerChanged(i128),          // For Integer, Float (slider)
    FloatChanged(i64),             // For Integer, Float (slider)
    Increase,                      // For Integer, Float (numeric up-down)
    Decrease,                      // For Integer, Float (numeric up-down)
    TextChanged(String),           // For Integer, Float, Text
    ApplyValue,                    // For Integer, Float, Text
    AddRow,                        // For Vector, Dictionary
    RemoveRow(usize),              // For Vector, Dictionary
    MoveRowUp(usize),              // For Vector, Dictionary
    MoveRowDown(usize),            // For Vector, Dictionary
    KeyTextChanged(usize, String), // For Dictionary
    ApplyKeyText(usize),           // For Dictionary
}

#[derive(Clone, Debug)]
pub struct SettingControlEvent {
    // Path of the control constructed during event bubbling in the drawing fuctions. The order of
    // the segments is reversed. Most controls add 0, except Section, Array, Vector and Dictionary,
    // which add the index of the child.
    pub path: Vec<usize>,

    pub event_type: SettingControlEventType,
}

pub struct InitData<S> {
    pub schema: S,
    pub trans: (), // todo
}

pub struct UpdatingData<'a> {
    pub path: Vec<usize>, // For SessionUpdated, the construction of the path is skipped

    pub event: SettingControlEventType,
    pub request_handler: &'a mut RequestHandler,

    // Path used to construct a command submitted with request_handler. For SessionUpdated, the
    // construction of the path is skipped
    pub string_path: String,
}

pub struct DrawingData {
    pub advanced: bool,
    pub common_trans: (), // todo
}

pub enum SettingControl {
    Section(Box<section::Control>),
    Choice(Box<choice::Control>),
    Optional(Box<optional::Control>),
    Switch(Box<switch::Control>),
    Boolean(boolean::Control),
    Integer(numeric::Control<i128>),
    Float(numeric::Control<f64>),
    Text(text::Control),
    Array(Box<array::Control>),
    Vector(Box<vector::Control>),
    Dictionary(Box<dictionary::Control>),
    HigherOrder(higher_order::Control),
    AudioDropdown(audio_dropdown::Control),
}

impl SettingControl {
    pub fn new(data: InitData<SchemaNode>) -> Self {
        let InitData { schema, trans } = data;

        match schema {
            SchemaNode::Section { entries } => {
                SettingControl::Section(Box::new(section::Control::new(InitData {
                    schema: entries,
                    trans,
                })))
            }
            SchemaNode::Choice { default, variants } => {
                SettingControl::Choice(Box::new(choice::Control::new(InitData {
                    schema: (default, variants),
                    trans,
                })))
            }
            SchemaNode::Optional {
                default_set,
                content,
            } => todo!(),
            SchemaNode::Switch {
                default_enabled,
                content_advanced,
                content,
            } => SettingControl::Switch(Box::new(switch::Control::new(InitData {
                schema: (default_enabled, content_advanced, content),
                trans,
            }))),
            SchemaNode::Boolean { default } => {
                SettingControl::Boolean(boolean::Control::new(InitData {
                    schema: default,
                    trans,
                }))
            }
            SchemaNode::Integer {
                default,
                min,
                max,
                step,
                gui,
            } => SettingControl::Integer(numeric::Control::new(InitData {
                schema: (default, min, max, step, gui),
                trans,
            })),
            SchemaNode::Float {
                default,
                min,
                max,
                step,
                gui,
            } => SettingControl::Float(numeric::Control::new(InitData {
                schema: (default, min, max, step, gui),
                trans,
            })),
            SchemaNode::Text { default } => SettingControl::Text(text::Control::new(InitData {
                schema: default,
                trans,
            })),
            SchemaNode::Array(entries) => {
                SettingControl::Array(Box::new(array::Control::new(InitData {
                    schema: entries,
                    trans,
                })))
            }
            SchemaNode::Vector {
                default_element,
                default,
            } => todo!(),
            SchemaNode::Dictionary {
                default_key,
                default_value,
                default,
            } => todo!(),
        }
    }

    pub fn update(&mut self, data: UpdatingData) {
        match self {
            SettingControl::Section(control) => control.update(data),
            SettingControl::Choice(control) => (),
            SettingControl::Optional(control) => (),
            SettingControl::Switch(control) => (),
            SettingControl::Boolean(control) => (),
            SettingControl::Integer(control) => (),
            SettingControl::Float(control) => (),
            SettingControl::Text(control) => (),
            SettingControl::Array(control) => (),
            SettingControl::Vector(control) => (),
            SettingControl::Dictionary(control) => (),
            SettingControl::HigherOrder(control) => (),
            SettingControl::AudioDropdown(control) => (),
        }
    }

    pub fn inline_element(&mut self, data: DrawingData) -> Element<SettingControlEvent> {
        todo!()
    }

    pub fn label_elements(&mut self, data: DrawingData) -> Vec<Element<SettingControlEvent>> {
        todo!()
    }

    pub fn control_elements(&mut self, data: DrawingData) -> Vec<Element<SettingControlEvent>> {
        todo!()
    }
}
