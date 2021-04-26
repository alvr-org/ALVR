use super::settings_controls::{setting_container, SettingProps};
use crate::{
    basic_components::{Button, ButtonType},
    session,
    translation::{use_trans, use_translation, SettingsTransNode, SettingsTransPathProvider},
};
use alvr_common::{data::SessionDesc, logging, prelude::*};
use serde_json as json;
use session::apply_session_settings;
use settings_schema::{EntryData, EntryType, SchemaNode};
use std::{collections::HashMap, rc::Rc};
use yew::{html, Callback, Properties};
use yew_functional::{
    function_component, use_context, use_effect_with_deps, use_state, ContextProvider,
};

#[derive(Clone, PartialEq)]
struct AdvancedContext(bool);

pub fn use_advanced() -> bool {
    use_context::<AdvancedContext>().unwrap().0
}

#[function_component(SettingsContent)]
pub fn settings_content(
    props: &SettingProps<Vec<(String, SchemaNode)>, HashMap<String, json::Value>>,
) -> Html {
    struct TabData {
        name: String,
        schema: SchemaNode,
        session: json::Value,
    }

    let (selected_tab_data, set_selected_tab_data) = {
        let (name, schema) = props.schema[0].clone();
        let session = props.session.get(&name).unwrap().clone();
        use_state(|| TabData {
            name,
            schema,
            session,
        })
    };

    let (advanced, set_advanced) = use_state(|| false);

    let tabs = props.schema.iter().map(|(name, schema)| {
        if let Some(session) = logging::show_err(trace_none!(props.session.get(name))).cloned() {
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
    });

    let content = setting_container(
        selected_tab_data.schema.clone(),
        selected_tab_data.session.clone(),
        {
            let selected_tab_data = selected_tab_data.clone();
            let session = props.session.clone();
            let set_session = props.set_session.clone();
            Callback::from(move |child_session| {
                let mut session = session.clone();
                session.insert(selected_tab_data.name.clone(), child_session);
                set_session.emit(session);
            })
        },
        *advanced,
    )
    .unwrap();

    html! {
        <SettingsTransPathProvider>
            <div style="border-bottom: 2px solid #eaeaea"> // <- todo use tailwind?
                <ul class="flex cursor-pointer">
                    {for tabs}
                </ul>
            </div>
            <Button // todo: put this on the right of the tab labels
                button_type=if *advanced {
                    ButtonType::Primary
                } else {
                    ButtonType::None
                }
                on_click={
                    let advanced = *advanced;
                    Callback::from(move |_| set_advanced(!advanced))
                }
            >
                {use_translation().get_attribute("settings", "advanced-mode")}
            </Button>
            <ContextProvider<AdvancedContext> context=AdvancedContext(*advanced)>
                <SettingsTransNode subkey=selected_tab_data.name.clone()>
                    {content}
                </SettingsTransNode>
            </ContextProvider<AdvancedContext>>
        </SettingsTransPathProvider>
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

    let set_session = {
        let theme = props.session.session_settings.extra.theme.variant.clone();
        Callback::from(move |session| {
            let theme = theme.clone();
            wasm_bindgen_futures::spawn_local(async move {
                logging::show_err(
                    async {
                        let session_settings =
                            trace_err!(json::from_value(trace_err!(json::to_value(session))?))?;
                        trace_err!(apply_session_settings(&session_settings).await)?;

                        if theme != session_settings.extra.theme.variant {
                            trace_err_dbg!(trace_none!(web_sys::window())?.location().reload())?;
                        }

                        StrResult::Ok(())
                    }
                    .await,
                );
            })
        })
    };

    if let Some(schema) = &*maybe_schema {
        if let Some(session_json) =
            logging::show_err(json::to_value(&props.session.session_settings))
        {
            if let Some(session) =
                logging::show_err(json::from_value::<HashMap<_, _>>(session_json))
            {
                return html! {
                    <SettingsContent schema=schema session=session set_session=set_session />
                };
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
