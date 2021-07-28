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
mod section;
mod switch;
mod text;
mod vector;

pub use audio_dropdown::*;
pub use boolean::*;
pub use choice::*;
pub use dictionary::*;
pub use help::*;
pub use higher_order::*;
pub use notice::*;
pub use numeric::*;
pub use optional::*;
pub use section::*;
pub use section::*;
pub use switch::*;
pub use text::*;
pub use vector::*;

use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;
use settings_schema::SchemaNode;

pub trait SettingControl {
    fn update(&mut self, ui: &mut Ui, session: json::Value) -> Option<DashboardResponse>;
}

pub trait SettingContainer {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse>;
}

pub struct EmptyControl;
impl SettingControl for EmptyControl {
    fn update(&mut self, _: &mut Ui, _: json::Value) -> Option<DashboardResponse> {
        None
    }
}

pub struct EmptyContainer;
impl SettingContainer for EmptyContainer {
    fn update(&mut self, _: &mut Ui, _: json::Value, _: bool) -> Option<DashboardResponse> {
        None
    }
}

// pub enum SettingControl {
//     None,
// }

// pub enum SettingContainer {
//     None,
//     Section(Section),
// }

// impl SettingContainer {
pub fn create_setting_control(schema: SchemaNode) -> Box<dyn SettingControl> {
    match schema {
        SchemaNode::Choice { default, variants } => todo!(),
        SchemaNode::Optional {
            default_set,
            content,
        } => todo!(),
        SchemaNode::Switch {
            default_enabled,
            content_advanced,
            content,
        } => todo!(),
        SchemaNode::Boolean { default } => todo!(),
        SchemaNode::Integer {
            default,
            min,
            max,
            step,
            gui,
        } => todo!(),
        SchemaNode::Float {
            default,
            min,
            max,
            step,
            gui,
        } => todo!(),
        SchemaNode::Text { default } => todo!(),
        _ => Box::new(EmptyControl),
    }
}

pub fn create_setting_container(schema: SchemaNode) -> Box<dyn SettingContainer> {
    match schema {
        SchemaNode::Section { entries } => Box::new(Section::new(entries)),
        SchemaNode::Choice { default, variants } => todo!(),
        SchemaNode::Optional {
            default_set,
            content,
        } => todo!(),
        SchemaNode::Switch {
            default_enabled,
            content_advanced,
            content,
        } => todo!(),
        SchemaNode::Boolean { default } => todo!(),
        SchemaNode::Integer {
            default,
            min,
            max,
            step,
            gui,
        } => todo!(),
        SchemaNode::Float {
            default,
            min,
            max,
            step,
            gui,
        } => todo!(),
        SchemaNode::Text { default } => todo!(),
        SchemaNode::Array(_) => todo!(),
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

//     pub fn update(session: &json::Value, advanced: bool) -> Option<DashboardResponse> {
//         None
//     }
// }
