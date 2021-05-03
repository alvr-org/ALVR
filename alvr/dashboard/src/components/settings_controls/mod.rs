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

pub use help::*;
pub use higher_order::*;
pub use notice::*;
pub use reset::*;

use alvr_common::{logging, prelude::*};
use array::Array;
use boolean::Boolean;
use choice::{choice_container, ChoiceControl};
use dictionary::Dictionary;
use numeric::{Float, Integer};
use optional::{optional_container, OptionalControl};
use section::Section;
use serde::Serialize;
use serde_json as json;
use settings_schema::{
    DictionaryDefault, OptionalDefault, SchemaNode, SwitchDefault, VectorDefault,
};
use std::collections::HashMap;
use switch::{switch_container, SwitchControl};
use text::Text;
use vector::Vector;
use yew::{html, Callback, Html, Properties};

#[derive(Properties, Clone, PartialEq)]
pub struct SettingProps<SCHEMA: Clone + PartialEq, SESSION: Clone + PartialEq> {
    pub schema: SCHEMA,
    pub session: SESSION,
    pub set_session: Callback<SESSION>,
}

fn bubble_up<T: Serialize>(set_session: Callback<json::Value>) -> Callback<T> {
    Callback::from(move |session| {
        if let Some(json) = logging::show_err(json::to_value(session)) {
            set_session.emit(json)
        }
    })
}

pub fn setting_control(
    schema: SchemaNode,
    session: json::Value,
    set_session: Callback<json::Value>,
) -> Option<Html> {
    logging::show_err((|| {
        StrResult::Ok(match schema {
            SchemaNode::Choice(schema) => Some(html! {
                <ChoiceControl
                    schema=schema
                    session=trace_err!(json::from_value::<HashMap<_, _>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Optional(schema) => Some(html! {
                <OptionalControl
                    schema=schema
                    session=trace_err!(json::from_value::<OptionalDefault<_>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Switch(schema) => Some(html! {
                <SwitchControl
                    schema=schema
                    session=trace_err!(json::from_value::<SwitchDefault<_>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Boolean(schema) => Some(html! {
                <Boolean
                    schema=schema
                    session=trace_err!(json::from_value::<bool>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Integer(schema) => Some(html! {
                <Integer
                    schema=schema
                    session=trace_err!(json::from_value::<i128>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Float(schema) => Some(html! {
                <Float
                    schema=schema
                    session=trace_err!(json::from_value::<f64>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Text(schema) => Some(html! {
                <Text
                    schema=schema
                    session=trace_err!(json::from_value::<String>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            _ => None,
        })
    })())
    .unwrap_or(None)
}

pub fn setting_container(
    schema: SchemaNode,
    session: json::Value,
    set_session: Callback<json::Value>,
    advanced: bool,
) -> Option<Html> {
    logging::show_err((|| {
        StrResult::Ok(match schema {
            SchemaNode::Section(schema) => Some(html! {
                <Section
                    schema=schema
                    session=trace_err!(json::from_value::<HashMap<_, _>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Choice(schema) => choice_container(
                schema,
                trace_err!(json::from_value::<HashMap<_, _>>(session))?,
                bubble_up(set_session),
                advanced,
            ),
            SchemaNode::Optional(schema) => optional_container(
                schema,
                trace_err!(json::from_value::<OptionalDefault<_>>(session))?,
                bubble_up(set_session),
                advanced,
            ),
            SchemaNode::Switch(schema) => switch_container(
                schema,
                trace_err!(json::from_value::<SwitchDefault<_>>(session))?,
                bubble_up(set_session),
                advanced,
            ),
            SchemaNode::Array(schema) => Some(html! {
                <Array
                    schema=schema
                    session=trace_err!(json::from_value::<Vec<_>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Vector(schema) => Some(html! {
                <Vector
                    schema=schema
                    session=trace_err!(json::from_value::<VectorDefault<_>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            SchemaNode::Dictionary(schema) => Some(html! {
                <Dictionary
                    schema=schema
                    session=trace_err!(json::from_value::<DictionaryDefault<_>>(session))?
                    set_session=bubble_up(set_session)
                />
            }),
            _ => None,
        })
    })())
    .unwrap_or(None)
}
