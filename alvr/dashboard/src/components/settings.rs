use super::settings_controls::SettingContainer;
use crate::{session, translation::use_trans};
use alvr_common::{data::SessionDesc, logging, prelude::*};
use serde_json as json;
use session::apply_session_settings;
use settings_schema::{EntryData, EntryType, SchemaNode};
use std::{collections::HashMap, rc::Rc};
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_effect_with_deps, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct SettingsContentProps {
    pub schema: Vec<(String, SchemaNode)>,
    pub session: HashMap<String, json::Value>,
}

#[function_component(SettingsContent)]
pub fn settings_content(props: &SettingsContentProps) -> Html {
    struct TabData {
        name: String,
        schema: SchemaNode,
        session: json::Value,
    }

    let (selected_tab_data, set_selected_tab_data) = {
        let (name, schema) = props.schema[0].clone();
        info!("{:?}", props.session);
        let session = props.session.get(&name).unwrap().clone();
        use_state(|| TabData {
            name,
            schema,
            session,
        })
    };

    let set_session = Callback::from(|session| {
        wasm_bindgen_futures::spawn_local(async {
            logging::show_err(
                async { apply_session_settings(&trace_err!(json::from_value(session))?).await }
                    .await,
            );
        })
    });

    html! {
        <>
            <div style="border-bottom: 2px solid #eaeaea"> // <- todo use tailwind?
                <ul class="flex cursor-pointer">
                    {
                        for props.schema.iter().map(|(name, schema)| {
                            if let Some(session) =
                                logging::show_err(trace_none!(props.session.get(name))).cloned()
                            {
                                let class = if selected_tab_data.name == *name {
                                    r"py-2 px-6 bg-white rounded-t-lg hover:shadow-md
                                    bg-gradient-to-tr from-blue-700 via-blue-700 to-blue-600
                                    hover:bg-blue-800 text-white shadow-md"
                                } else {
                                    "py-2 px-6 bg-white rounded-t-lg hover:shadow-md"
                                };

                                let on_click = {
                                    let name = name.clone();
                                    let schema = schema.clone();
                                    let session = session.clone();
                                    let set_selected_tab_data = Rc::clone(&set_selected_tab_data);
                                    Callback::from(move |_| {
                                        set_selected_tab_data(TabData {
                                            name: name.clone(),
                                            schema: schema.clone(),
                                            session: session.clone(),
                                        })
                                    })
                                };

                                html! {
                                    <li key=name.as_ref() class=class onclick=on_click>
                                        {use_trans(name)}
                                    </li>
                                }
                            } else {
                                html!()
                            }

                        })
                    }
                </ul>
            </div>
            <div>
                <SettingContainer
                    schema=selected_tab_data.schema.clone()
                    session=selected_tab_data.session.clone()
                    set_session=set_session
                />
            </div>
        </>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct SettingsProps {
    pub session: Rc<SessionDesc>,
}

#[function_component(Settings)]
pub fn settings(props: &SettingsProps) -> Html {
    let (maybe_schema, set_schema) = use_state(|| None);

    use_effect_with_deps(
        move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                logging::show_err_async(async {
                    let schema = trace_err!(session::fetch_schema().await)?;
                    if let SchemaNode::Section(entries) = schema {
                        let schema = entries
                            .into_iter()
                            .filter_map(|(name, content)| {
                                if let EntryType::Data(EntryData { content, .. }) = content {
                                    Some((name, content))
                                } else {
                                    error!("Invalid schema!");
                                    None
                                }
                            })
                            .collect();

                        set_schema(Some(schema));
                    } else {
                        error!("Invalid schema!");
                    }

                    StrResult::Ok(())
                })
                .await;
            });

            || ()
        },
        (),
    );

    if let Some(schema) = &*maybe_schema {
        if let Some(session_json) =
            logging::show_err(json::to_value(&props.session.session_settings))
        {
            if let Some(session) =
                logging::show_err(json::from_value::<HashMap<_, _>>(session_json))
            {
                return html!(<SettingsContent schema=schema session=session />);
            } else {
                html!()
            }
        } else {
            html!()
        }
    } else {
        html!("Loading...")
    }
}
