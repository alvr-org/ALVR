use wasm_bindgen::prelude::*;
use yew::{html, Children, Properties};
use yew_functional::{function_component, use_context, ContextProvider};

#[wasm_bindgen]
extern "C" {
    fn trans_key_exists(key: &str) -> bool;
    pub fn t(key: &str) -> String;
    pub async fn change_language(code: &str);
}

#[derive(Properties, Clone, PartialEq)]
pub struct TransProviderProps {
    pub children: Children,
}

#[function_component(TransProvider)]
pub fn trans_provider(props: &TransProviderProps) -> Html {
    html! {
        <ContextProvider<Vec<String>> context=vec![]>
            {props.children.clone()}
        </ContextProvider<Vec<String>>>
    }
}

pub enum TransKeysName {
    Found(String),
    NotFound { last_segment: String },
}

pub struct TransKeys {
    name: TransKeysName,
    help: Option<String>,
    notice: Option<String>,
}

pub fn use_trans_keys(subkey: impl Into<Option<String>>) -> TransKeys {
    let mut route_segments = (*use_context::<Vec<String>>().expect("Trans context")).clone();

    if let Some(subkey) = subkey.into() {
        route_segments.push(subkey.to_owned())
    }
    let route = route_segments.join(".");

    if trans_key_exists(&route) && !trans_key_exists(&(route.clone() + ".name")) {
        TransKeys {
            name: TransKeysName::Found(route),
            help: None,
            notice: None,
        }
    } else {
        let name_key = if trans_key_exists(&(route.clone() + ".name")) {
            TransKeysName::Found(route.clone() + ".name")
        } else {
            TransKeysName::NotFound {
                last_segment: route_segments.last().cloned().unwrap_or("???".into()),
            }
        };

        let help_key = route.clone() + ".help";
        let notice_key = route + ".notice";

        TransKeys {
            name: name_key,
            help: trans_key_exists(&help_key).then(|| help_key),
            notice: trans_key_exists(&notice_key).then(|| notice_key),
        }
    }
}

pub struct TransValues {
    name: String,
    help: Option<String>,
    notice: Option<String>,
}

pub fn use_trans(subkey: impl Into<Option<String>>) -> TransValues {
    let keys = use_trans_keys(subkey);

    let name = match keys.name {
        TransKeysName::Found(key) => t(&key),
        TransKeysName::NotFound { last_segment } => last_segment,
    };

    TransValues {
        name,
        help: keys.help.map(|key| t(&key)),
        notice: keys.notice.map(|key| t(&key)),
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct TransNameProps {
    #[prop_or_default]
    subkey: Option<String>,
}

#[function_component(TransName)]
pub fn trans_name(props: &TransNameProps) -> Html {
    let values = use_trans(props.subkey.clone());

    html!({ values.name })
}
