use super::SettingProps;
use serde_json as json;
use settings_schema::SchemaChoice;
use std::collections::HashMap;
use yew::{html, Callback, Html};
use yew_functional::function_component;

fn get_variant(entries: &HashMap<String, json::Value>) -> String {
    entries["variant"].as_str().unwrap().to_owned()
}

#[function_component(ChoiceControl)]
pub fn choice_control(props: &SettingProps<SchemaChoice, HashMap<String, json::Value>>) -> Html {
    html!("choice control")
}

pub fn choice_container(
    schema: SchemaChoice,
    session: HashMap<String, json::Value>,
    set_session: Callback<HashMap<String, json::Value>>,
    advanced: bool,
) -> Option<Html> {
    let variant = get_variant(&session);
    let maybe_data = schema
        .variants
        .into_iter()
        .find(|(name, _)| *name == variant)
        .map(|(_, data)| data)
        .unwrap();

    if let Some(entry) = maybe_data {
        if advanced || !entry.advanced {
            super::setting_container(
                entry.content,
                session[&variant].clone(),
                Callback::from(move |child_session| {
                    let mut session = session.clone();
                    session.insert(variant.clone(), child_session);
                    set_session.emit(session);
                }),
                advanced,
            )
        } else {
            None
        }
    } else {
        None
    }
}
