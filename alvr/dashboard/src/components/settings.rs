use crate::session;
use alvr_common::{data::SessionDesc, logging, prelude::*};
use std::rc::Rc;
use yew::{html, Properties};
use yew_functional::{function_component, use_effect_with_deps, use_state};

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
            {"settings"}
        }
    } else {
        html!("Loading...")
    }
}
