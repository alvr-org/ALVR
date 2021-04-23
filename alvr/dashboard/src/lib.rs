// during development
#![allow(dead_code)]

mod basic_components;
mod components;
mod dashboard;
mod events_dispatch;
mod logging_backend;
mod session;
mod translation;

use alvr_common::{logging, prelude::*};
use dashboard::Dashboard;
use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use translation::{TransContext, TransProvider, TranslationManager};
use wasm_bindgen::prelude::*;
use yew::{html, Callback};
use yew_functional::{function_component, use_effect_with_deps, use_state};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub fn get_id() -> String {
    format!("alvr{}", ID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn get_base_url() -> String {
    format!(
        "http://{}",
        web_sys::window().unwrap().location().host().unwrap()
    )
}

#[function_component(Root)]
fn root() -> Html {
    let (maybe_data, set_data) = use_state(|| None);

    let update_session_async = Callback::from(move |()| {
        let set_data = Rc::clone(&set_data);
        wasm_bindgen_futures::spawn_local(async move {
            logging::show_err_async(async {
                let session = session::fetch_session().await?;

                let translation_manager =
                    TranslationManager::new(session.to_settings().extra.language).await?;

                set_data(Some((Rc::new(session), Rc::new(translation_manager))));

                StrResult::Ok(())
            })
            .await;
        });
    });

    use_state({
        let update_session_async = update_session_async.clone();
        || recv_event_cb!(SessionUpdated, || update_session_async.emit(()))
    });

    // run only once
    use_effect_with_deps(
        move |_| {
            update_session_async.emit(());
            || ()
        },
        (),
    );

    if let Some((session, translation_manager)) = &*maybe_data {
        html! {
            <TransProvider context=TransContext { manager: translation_manager.clone() }>
                <Dashboard session=Rc::clone(session) />
            </TransProvider>
        }
    } else {
        html!(<h1 class="position-absolute top-50 start-50 translate-middle">{"Loading..."}</h1>)
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    logging_backend::init();

    wasm_bindgen_futures::spawn_local(async {
        logging::show_err_async(events_dispatch::events_dispatch_loop()).await;
    });

    yew::start_app::<Root>();
}
