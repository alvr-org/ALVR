use wasm_bindgen::prelude::*;
use yew::{html, Children, Properties};
use yew_functional::{function_component, use_context, ContextProvider};

#[wasm_bindgen]
extern "C" {
    pub async fn change_language(code: &str);
    fn trans_key_exists(key: &str) -> bool;
    pub fn t(key: &str) -> String;
}

#[derive(Clone, PartialEq)]
struct TransContext(Vec<String>);

#[derive(Properties, Clone, PartialEq)]
pub struct TransProviderProps {
    children: Children,
}

#[function_component(TransProvider)]
pub fn trans_provider(props: &TransProviderProps) -> Html {
    html! {
        <ContextProvider<TransContext> context=TransContext(vec![])>
            {props.children.clone()}
        </ContextProvider<TransContext>>
    }
}

pub enum TransKeysName {
    Found(String),
    NotFound { path: Vec<String> },
}

pub struct TransKeys {
    name: TransKeysName,
    help: Option<String>,
    notice: Option<String>,
}

pub fn use_trans_keys(subkey: Option<String>) -> TransKeys {
    let context = use_context::<TransContext>().expect("TransContext");

    // context.
    todo!()
}

pub struct TransValues {
    name: String,
    help: Option<String>,
    notice: Option<String>,
}

pub fn use_trans(subkey: Option<String>) -> TransValues {
    todo!()
}
