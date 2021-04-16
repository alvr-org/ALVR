use alvr_common::{
    data::{SessionDesc, SessionSettings},
    prelude::*,
};
use settings_schema::SchemaNode;

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

pub async fn apply_session_settings(settings: &SessionSettings) -> StrResult {
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

pub async fn apply_session_settings_raw(settings: String) -> StrResult {
    trace_err!(
        reqwest::Client::new()
            .post(format!(
                "{}/api/session/store-settings",
                crate::get_base_url()
            ))
            .body(settings)
            .send()
            .await
    )?;

    Ok(())
}
