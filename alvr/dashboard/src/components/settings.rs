use crate::{session, translation::use_trans};
use alvr_common::{data::SessionDesc, logging, prelude::*};
use settings_schema::{EntryData, EntryType, SchemaNode};
use std::rc::Rc;
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_effect_with_deps, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct SettingsContentProps {
    pub schema: Vec<(String, SchemaNode)>,
    pub session: Rc<SessionDesc>,
}

#[function_component(SettingsContent)]
pub fn settings_content(props: &SettingsContentProps) -> Html {
    let (selected_tab, set_selected_tab) = {
        let initial_tab = props.schema[0].0.clone();
        use_state(|| initial_tab)
    };

    html! {
        <>
            <div style="border-bottom: 2px solid #eaeaea"> // <- todo use tailwind?
                <ul class="flex cursor-pointer">
                    {
                        for props.schema.iter().map(|(name, _)| {
                            let class = if *selected_tab == *name {
                                format!(
                                    "py-2 px-6 bg-white rounded-t-lg hover:shadow-md {} {}",
                                    "bg-gradient-to-tr from-blue-700 via-blue-700 to-blue-600",
                                    "hover:bg-blue-800 text-white shadow-md"
                                )
                            } else {
                                "py-2 px-6 bg-white rounded-t-lg hover:shadow-md".to_owned()
                            };

                            let on_click = {
                                let name = name.clone();
                                let set_selected_tab = Rc::clone(&set_selected_tab);
                                Callback::from(move |_| set_selected_tab(name.clone()))
                            };

                            html! {
                                <li key=name.as_ref() class=class onclick=on_click>
                                    {use_trans(name)}
                                </li>
                            }
                        })
                    }
                </ul>
            </div>
            <div>

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
        html!(<SettingsContent schema=schema session=Rc::clone(&props.session) />)
    } else {
        html!("Loading...")
    }
}
