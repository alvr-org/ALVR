use super::SettingProps;
use crate::translation::{use_setting_trans, SettingsTrans};
use serde_json as json;
use settings_schema::EntryType;
use std::collections::HashMap;
use yew::{html, Callback};
use yew_functional::function_component;

#[function_component(Entry)]
fn entry(props: &SettingProps<(String, EntryType), Option<json::Value>>) -> Html {
    let SettingsTrans { name, .. } = use_setting_trans(&props.schema.0);
    html! {
        <div>
            {name}
        </div>
    }
}

#[function_component(Section)]
pub fn section(
    props: &SettingProps<Vec<(String, EntryType)>, HashMap<String, json::Value>>,
) -> Html {
    html! {
        <>
            {
                for props.schema.iter().map(|(name, schema)| {
                    let name = name.clone();
                    let session = props.session.clone();
                    let set_session = props.set_session.clone();
                    html! {
                        <Entry
                            // key=name.clone() <- bug: VComp is not mounted
                            schema=(name.clone(), schema.clone())
                            session=session.get(&name).cloned()
                            set_session={
                                let name = name.clone();
                                Callback::from(move |child_session| {
                                    if let Some(json) = child_session {
                                        let mut session = session.clone();
                                        session.insert(name.clone(), json);
                                        set_session.emit(session);
                                    }
                                })
                            }
                        />
                    }
                })
            }
        </>
    }
}
