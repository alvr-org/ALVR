use super::{Help, HigherOrder, Notice, SettingProps};
use crate::translation::{use_setting_trans, SettingsTrans};
use alvr_common::data::SessionDesc;
use serde_json as json;
use settings_schema::EntryType;
use std::collections::HashMap;
use yew::{html, Callback, Html, Properties};
use yew_functional::{function_component, use_context};

#[derive(Properties, Clone, PartialEq)]
struct EntryProps {
    name: String,
    control: Option<Html>,
    container: Option<Html>,
}

#[function_component(Entry)]
fn entry(props: &EntryProps) -> Html {
    let SettingsTrans { name, help, notice } = use_setting_trans(&props.name);

    html! {
        <div class="flex flex-col">
            <div class="flex flex-row">
                {name}
                {
                    if let Some(help) = help.clone() {
                        html!(<Help text=help />)
                    } else {
                        html!()
                    }
                }
                {
                    if let Some(control) = props.control.clone() {
                        control
                    } else {
                        html!()
                    }
                }
            </div>
            {
                if let Some(notice) = notice.clone() {
                    html!(<Notice text=notice />)
                } else {
                    html!()
                }
            }
            {
                if let Some(container) = props.container.clone() {
                    html! {
                        <div class="ml-10">
                            {container}
                        </div>
                    }
                } else {
                    html!()
                }
            }
        </div>
    }
}

#[function_component(Section)]
pub fn section(
    props: &SettingProps<Vec<(String, EntryType)>, HashMap<String, json::Value>>,
) -> Html {
    let advanced = use_context::<SessionDesc>()
        .unwrap()
        .session_settings
        .extra
        .show_advanced;

    let entries = props.schema.iter().filter_map(|(name, schema)| {
        let maybe_control;
        let maybe_container;
        match schema {
            EntryType::Data(data) => {
                if advanced || !data.advanced {
                    let set_session = {
                        let name = name.clone();
                        let session = props.session.clone();
                        let set_session = props.set_session.clone();
                        Callback::from(move |child_session| {
                            let mut session = session.clone();
                            session.insert(name.clone(), child_session);
                            set_session.emit(session);
                        })
                    };
                    maybe_control = super::setting_control(
                        data.content.clone(),
                        props.session[name].clone(),
                        set_session.clone(),
                    );
                    maybe_container = super::setting_container(
                        data.content.clone(),
                        props.session[name].clone(),
                        set_session,
                        advanced,
                    );
                } else {
                    maybe_control = None;
                    maybe_container = None;
                }
            }
            EntryType::HigherOrder {
                data_type,
                modifiers,
            } => {
                maybe_control = (!advanced)
                    .then(|| html!(<HigherOrder data_type=data_type modifiers=modifiers />));
                maybe_container = None;
            }
            EntryType::Placeholder => {
                maybe_control = Some(html!("todo placeholder"));
                maybe_container = None;
            }
        }

        if maybe_control.is_some() || maybe_container.is_some() {
            Some(html! {
                <Entry
                    // key=name.clone() // <- bug: VComp is not mounted
                    name=name.clone()
                    control=maybe_control
                    container=maybe_container
                />
            })
        } else {
            None
        }
    });

    html!(for entries)
}
