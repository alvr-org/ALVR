use crate::{session, translation::use_trans};
use alvr_common::{data::SessionDesc, logging, prelude::*};
use settings_schema::{EntryType, SchemaNode};
use std::rc::Rc;
use yew::{html, virtual_dom::VNode, Html, Properties};
use yew_functional::{function_component, use_effect_with_deps, use_state};

fn generate_settings(schema: &SchemaNode, depth: i32) -> VNode {
    let sections: Vec<VNode> = Vec::new();
    log::info!("{:?}", schema);
    match schema {
        SchemaNode::Section(entries) => {
            for (name, data) in entries.iter() {
                log::info!("{}", name);
            }
            if depth == 0 {
                return html! {
                    <div style="border-bottom: 2px solid #eaeaea">
                        <ul class="flex cursor-pointer">
                            { for entries.iter().enumerate().map(|(index, (name, data))| if index == 0 {
                                html! {
                                    <li class="py-2 px-6 bg-white rounded-t-lg hover:shadow-md bg-gradient-to-tr from-blue-700 via-blue-700 to-blue-600 hover:bg-blue-800 text-white shadow-md">
                                        {use_trans(name)}
                                    </li>
                                }
                            } else {
                                html! {
                                    <li class="py-2 px-6 bg-white rounded-t-lg hover:shadow-md">
                                        {use_trans(name)}
                                    </li>
                                }
                            }) }

                        </ul>
                    </div>
                };
            } else {
                return html! {};
            }
        }
        SchemaNode::Choice {
            default,
            variants,
            gui,
        } => html! {},
        SchemaNode::Optional {
            default_set,
            content,
        } => html! {},
        SchemaNode::Switch {
            default_enabled,
            content_advanced,
            content,
        } => html! {},
        SchemaNode::Boolean { default } => html! {},
        SchemaNode::Integer {
            default,
            min,
            max,
            step,
            gui,
        } => html! {},
        SchemaNode::Float {
            default,
            min,
            max,
            step,
            gui,
        } => html! {},
        SchemaNode::Text { default } => html! {},
        SchemaNode::Array(_) => html! {},
        SchemaNode::Vector {
            default_element,
            default,
        } => html! {},
        SchemaNode::Dictionary {
            default_key,
            default_value,
            default,
        } => html! {},
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub session: Rc<SessionDesc>,
}

#[function_component(Settings)]
pub fn settings(Props { session }: &Props) -> Html {
    let (maybe_schema, set_schema) = use_state(|| None);
    use_effect_with_deps(
        move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                logging::show_err_async(async {
                    set_schema(Some(trace_err!(session::fetch_schema().await)?));

                    StrResult::Ok(())
                })
                .await;
            });

            || ()
        },
        (),
    );
    if let Some(schema) = &*maybe_schema {
        html! {
            <>
                {generate_settings(schema, 0)}
            </>
        }
    } else {
        html!("Loading...")
    }
}
