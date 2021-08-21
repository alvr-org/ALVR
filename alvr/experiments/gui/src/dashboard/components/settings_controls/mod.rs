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
pub use section::*;
pub use switch::*;
pub use text::*;
pub use vector::*;

use crate::translation::{SharedTranslation, TranslationBundle};
use egui::Ui;
use serde::Serialize;
use serde_json as json;
use settings_schema::SchemaNode;
use std::sync::{atomic::AtomicUsize, Arc};

fn get_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// Due to the nature of immediate mode GUIs, the parent containers cannot conditionally render based
// on the presence of the child container, so it is the child responsibility to format the container
pub fn container<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        // Indentation
        ui.add_space(20_f32);

        content(ui)
    })
    .inner
}

pub fn into_fragment<T: Serialize>(fragment: T) -> SettingsResponse {
    SettingsResponse::SessionFragment(json::to_value(fragment).unwrap())
}

pub enum SettingsResponse {
    SessionFragment(json::Value),
    PresetInvocation(String),
}
pub fn map_fragment<T: Serialize>(
    res: Option<SettingsResponse>,
    map: impl FnOnce(json::Value) -> T,
) -> Option<SettingsResponse> {
    match res {
        Some(SettingsResponse::SessionFragment(fragment)) => {
            Some(super::into_fragment(map(fragment)))
        }
        res => res,
    }
}

pub struct SettingsContext {
    pub advanced: bool,
    pub view_width: f32,
    pub t: Arc<SharedTranslation>,
}

pub trait SettingControl {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        ctx: &SettingsContext,
    ) -> Option<SettingsResponse>;
}

pub trait SettingContainer {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        ctx: &SettingsContext,
    ) -> Option<SettingsResponse>;
}

pub struct EmptyControl;
impl SettingControl for EmptyControl {
    fn ui(&mut self, _: &mut Ui, _: json::Value, _: &SettingsContext) -> Option<SettingsResponse> {
        None
    }
}

pub struct EmptyContainer;
impl SettingContainer for EmptyContainer {
    fn ui(&mut self, _: &mut Ui, _: json::Value, _: &SettingsContext) -> Option<SettingsResponse> {
        None
    }
}

pub fn create_setting_control(
    schema: SchemaNode,
    session_fragment: json::Value,
    trans_path: &str,
    trans: &TranslationBundle,
) -> Box<dyn SettingControl> {
    match schema {
        SchemaNode::Choice { default, variants } => Box::new(ChoiceControl::new(
            default,
            variants,
            session_fragment,
            trans_path,
            trans,
        )),
        SchemaNode::Optional {
            default_set,
            content,
        } => Box::new(EmptyControl),
        SchemaNode::Switch {
            default_enabled,
            content_advanced,
            content,
        } => Box::new(SwitchControl::new(
            default_enabled,
            content_advanced,
            *content,
            session_fragment,
            trans_path,
            trans,
        )),
        SchemaNode::Boolean { default } => Box::new(Boolean::new(default)),
        SchemaNode::Integer {
            default,
            min,
            max,
            step,
            gui,
        } => Box::new(NumericWidget::new(
            // todo: PR for i128 support for emath::Numeric trait
            session_fragment,
            default as i64,
            min.map(|n| n as i64),
            max.map(|n| n as i64),
            step.map(|n| n as i64),
            gui,
            true,
        )),
        SchemaNode::Float {
            default,
            min,
            max,
            step,
            gui,
        } => Box::new(NumericWidget::new(
            session_fragment,
            default,
            min,
            max,
            step,
            gui,
            false,
        )),
        SchemaNode::Text { default } => Box::new(Text::new(default, session_fragment)),
        _ => Box::new(EmptyControl),
    }
}

pub fn create_setting_container(
    schema: SchemaNode,
    session_fragment: json::Value,
    trans_path: &str,
    trans: &TranslationBundle,
) -> Box<dyn SettingContainer> {
    match schema {
        SchemaNode::Section { entries } => {
            Box::new(Section::new(entries, session_fragment, trans_path, trans))
        }
        SchemaNode::Choice { default, variants } => Box::new(ChoiceContainer::new(
            variants,
            session_fragment,
            trans_path,
            trans,
        )),
        SchemaNode::Optional {
            default_set,
            content,
        } => Box::new(EmptyContainer),
        SchemaNode::Switch {
            default_enabled,
            content_advanced,
            content,
        } => Box::new(SwitchContainer::new(
            content_advanced,
            *content,
            session_fragment,
            trans_path,
            trans,
        )),
        SchemaNode::Array(array) => {
            Box::new(Array::new(array, session_fragment, trans_path, trans))
        }
        SchemaNode::Vector {
            default_element,
            default,
        } => Box::new(EmptyContainer),
        SchemaNode::Dictionary {
            default_key,
            default_value,
            default,
        } => Box::new(EmptyContainer),
        _ => Box::new(EmptyContainer),
    }
}
