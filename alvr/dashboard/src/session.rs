use alvr_common::{
    data::{SessionDesc, SessionSettings},
    prelude::*,
};
use serde::Serialize;
use settings_schema::SchemaNode;
use yew::{html, Children, Properties};
use yew_functional::{function_component, ContextProvider};

pub async fn fetch_schema() -> StrResult<SchemaNode> {
    trace_err!(
        trace_err!(reqwest::get(format!("{}/api/settings-schema", crate::get_base_url())).await)?
            .json()
            .await
    )
}

pub async fn fetch_session() -> StrResult<SessionDesc> {
    trace_err!(
        trace_err!(reqwest::get(format!("{}/api/session/load", crate::get_base_url())).await)?
            .json()
            .await
    )
}

async fn apply_session_settings_impl<T: Serialize>(settings: &T) -> StrResult {
    trace_err!(
        reqwest::Client::new()
            .post(format!(
                "{}/api/session/store-settings",
                crate::get_base_url()
            ))
            .json(settings)
            .send()
            .await
    )?;

    Ok(())
}

pub async fn apply_session_settings(settings: &SessionSettings) -> StrResult {
    apply_session_settings_impl(settings).await
}

pub async fn apply_session_settings_raw(settings: &serde_json::Value) -> StrResult {
    apply_session_settings_impl(settings).await
}

#[derive(Properties, Clone, PartialEq)]
pub struct SessionProviderProps {
    pub initial_session: SessionDesc,
    pub children: Children,
}

#[function_component(SessionProvider)]
pub fn session_provider(props: &SessionProviderProps) -> Html {
    html! {
        <ContextProvider<SessionDesc> context=props.initial_session.clone()>
            {props.children.clone()}
        </ContextProvider<SessionDesc>>
    }
}
