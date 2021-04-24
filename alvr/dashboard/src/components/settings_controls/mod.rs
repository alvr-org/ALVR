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
use choice::{ChoiceContainer, ChoiceControl};
use dictionary::Dictionary;
use numeric::{Float, Integer};
use optional::{OptionalContainer, OptionalControl};
use section::Section;
use serde::Serialize;
use serde_json as json;
use settings_schema::{
    DictionaryDefault, OptionalDefault, SchemaNode, SwitchDefault, VectorDefault,
};
use std::collections::HashMap;
use switch::{SwitchContainer, SwitchControl};
use text::Text;
use vector::Vector;
use yew::{html, Callback, Properties};
use yew_functional::function_component;

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

#[function_component(SettingControl)]
pub fn setting_control(props: &SettingProps<SchemaNode, json::Value>) -> Html {
    let session = props.session.clone();
    let set_session = props.set_session.clone();

    logging::show_err((|| {
        StrResult::Ok(match &props.schema {
            SchemaNode::Choice(schema) => html! {
                <ChoiceControl
                    schema=schema
                    session=trace_err!(json::from_value::<HashMap<_, _>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Optional(schema) => html! {
                <OptionalControl
                    schema=schema
                    session=trace_err!(json::from_value::<OptionalDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Switch(schema) => html! {
                <SwitchControl
                    schema=schema
                    session=trace_err!(json::from_value::<SwitchDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Boolean(schema) => html! {
                <Boolean
                    schema=schema
                    session=trace_err!(json::from_value::<bool>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Integer(schema) => html! {
                <Integer
                    schema=schema
                    session=trace_err!(json::from_value::<i128>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Float(schema) => html! {
                <Float
                    schema=schema
                    session=trace_err!(json::from_value::<f64>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Text(schema) => html! {
                <Text
                    schema=schema
                    session=trace_err!(json::from_value::<String>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            _ => html!(),
        })
    })())
    .unwrap_or_else(|| html!())
}

#[function_component(SettingContainer)]
pub fn setting_container(props: &SettingProps<SchemaNode, json::Value>) -> Html {
    let session = props.session.clone();
    let set_session = props.set_session.clone();

    logging::show_err((|| {
        StrResult::Ok(match &props.schema {
            SchemaNode::Section(schema) => html! {
                <Section
                    schema=schema
                    session=trace_err!(json::from_value::<HashMap<_, _>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Choice(schema) => html! {
                <ChoiceContainer
                    schema=schema
                    session=trace_err!(json::from_value::<HashMap<_, _>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Optional(schema) => html! {
                <OptionalContainer
                    schema=schema
                    session=trace_err!(json::from_value::<OptionalDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Switch(schema) => html! {
                <SwitchContainer
                    schema=schema
                    session=trace_err!(json::from_value::<SwitchDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Array(schema) => html! {
                <Array
                    schema=schema
                    session=trace_err!(json::from_value::<Vec<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Vector(schema) => html! {
                <Vector
                    schema=schema
                    session=trace_err!(json::from_value::<VectorDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            SchemaNode::Dictionary(schema) => html! {
                <Dictionary
                    schema=schema
                    session=trace_err!(json::from_value::<DictionaryDefault<json::Value>>(session))?
                    set_session=bubble_up(set_session)
                />
            },
            _ => html!(),
        })
    })())
    .unwrap_or_else(|| html!())
}
