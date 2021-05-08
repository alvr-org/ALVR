use super::settings_controls::setting_container;
use crate::{
    basic_components::{Button, ButtonType},
    session,
    translation::{use_translation, SettingsTransNode, SettingsTransPathProvider},
};
use alvr_common::{data::SessionDesc, logging, prelude::*};
use serde_json as json;
use settings_schema::{EntryData, EntryType, SchemaNode};
use std::collections::HashMap;
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_context, use_effect_with_deps, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct SettingsContentProps {
    schema: Vec<(String, SchemaNode)>,
}

#[function_component(SettingsContent)]
pub fn settings_content(props: &SettingsContentProps) -> Html {
    let session = use_context::<SessionDesc>().unwrap();
    let session_settings = session.session_settings;
    let advanced = session_settings.extra.show_advanced;
    let session_map = json::from_value::<HashMap<String, json::Value>>(
        json::to_value(&session_settings).unwrap(),
    )
    .unwrap();

    let t = use_translation();

    struct TabData {
        name: String,
        schema: SchemaNode,
    }

    let selected_tab_data = {
        let (name, schema) = props.schema[0].clone();
        use_state(|| TabData { name, schema })
    };

    let tabs = props.schema.iter().map(|(name, schema)| {
        let class = if selected_tab_data.name == *name {
            "py-2 px-6 bg-white rounded-t-lg hover:shadow-md
                bg-gradient-to-tr from-blue-700 via-blue-700 to-blue-600
                hover:bg-blue-800 text-white shadow-md"
        } else {
            "py-2 px-6 bg-white rounded-t-lg hover:shadow-md"
        };

        let on_click = {
            let name = name.clone();
            let schema = schema.clone();
            let selected_tab_data = selected_tab_data.clone();
            Callback::from(move |_| {
                selected_tab_data.set(TabData {
                    name: name.clone(),
                    schema: schema.clone(),
                })
            })
        };

        html! {
            <li key=name.as_ref() class=class onclick=on_click>
                {t.get(name)}
            </li>
        }
    });

    let content = setting_container(
        selected_tab_data.schema.clone(),
        session_map[&selected_tab_data.name].clone(),
        {
            let selected_tab_data = selected_tab_data.clone();
            let session = session_map.clone();
            let theme = session_settings.extra.theme.variant.clone();
            Callback::from(move |child_session| {
                let mut session = session.clone();
                session.insert(selected_tab_data.name.clone(), child_session);

                let theme = theme.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    logging::show_err(
                        async {
                            let session_settings =
                                trace_err!(json::from_value(trace_err!(json::to_value(session))?))?;
                            trace_err!(session::apply_session_settings(&session_settings).await)?;

                            if theme != session_settings.extra.theme.variant {
                                trace_err_dbg!(trace_none!(web_sys::window())?
                                    .location()
                                    .reload())?;
                            }

                            StrResult::Ok(())
                        }
                        .await,
                    );
                })
            })
        },
        advanced,
    )
    .unwrap();

    let advanced_on_click = Callback::from(move |_| {
        let mut session_settings = session_settings.clone();
        session_settings.extra.show_advanced = !advanced;
        wasm_bindgen_futures::spawn_local(async move {
            logging::show_err(session::apply_session_settings(&session_settings).await);
        });
    });

    html! {
        <SettingsTransPathProvider>
            <div class="border-b-2 border-gray-200">
                <ul class="flex cursor-pointer">
                    {for tabs}
                </ul>
            </div>
            <Button // todo: put this on the right of the tab labels
                button_type=if advanced {
                    ButtonType::Primary
                } else {
                    ButtonType::None
                }
                on_click=advanced_on_click
            >
                {use_translation().attribute("settings", "advanced-mode")}
            </Button>
            <SettingsTransNode subkey=selected_tab_data.name.clone()>
                <div class="h-fill overflow-y-auto">
                    {content}
                </div>
            </SettingsTransNode>
        </SettingsTransPathProvider>
    }
}

#[function_component(Settings)]
pub fn settings() -> Html {
    let maybe_schema_handle = use_state(|| None);

    use_effect_with_deps(
        {
            let maybe_schema_handle = maybe_schema_handle.clone();
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

                            maybe_schema_handle.set(Some(schema));
                        } else {
                            error!("Invalid schema!");
                        }

                        StrResult::Ok(())
                    })
                    .await;
                });

                || ()
            }
        },
        (),
    );

    if let Some(schema) = &*maybe_schema_handle {
        return html! {
            <SettingsContent schema=schema />
        };
    } else {
        html!("Loading...")
    }
}
