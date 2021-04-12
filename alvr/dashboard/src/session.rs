use alvr_common::{
    data::{SessionDesc, SessionSettings},
    prelude::*,
};
use serde::{de::DeserializeOwned, Serialize};
use settings_schema::SchemaNode;
use std::marker::PhantomData;
use yewtil::fetch::{self, create_request, FetchRequest, Json, MethodBody};

async fn fetch_get<T: DeserializeOwned>(url: &str) -> StrResult<T> {
    struct Request<T> {
        url: String,
        _phantom: PhantomData<T>,
    }

    impl<T: DeserializeOwned> FetchRequest for Request<T> {
        type RequestBody = ();
        type ResponseBody = T;
        type Format = Json;

        fn url(&self) -> String {
            self.url.clone()
        }

        fn method(&self) -> MethodBody<()> {
            MethodBody::Get
        }

        fn headers(&self) -> Vec<(String, String)> {
            vec![]
        }
    }

    let maybe_request = create_request(&Request::<T> {
        url: url.into(),
        _phantom: PhantomData,
    });

    trace_err!(fetch::fetch_resource::<Request<_>>(maybe_request, PhantomData).await)
}

async fn fetch_post<T: Serialize>(url: &str, body: T) -> StrResult {
    struct Request<T> {
        url: String,
        body: T,
    }

    impl<T: Serialize> FetchRequest for Request<T> {
        type RequestBody = T;
        type ResponseBody = ();
        type Format = Json;

        fn url(&self) -> String {
            self.url.clone()
        }

        fn method(&self) -> MethodBody<T> {
            MethodBody::Post(&self.body)
        }

        fn headers(&self) -> Vec<(String, String)> {
            vec![]
        }
    }

    let maybe_request = create_request(&Request {
        url: url.into(),
        body,
    });

    trace_err!(fetch::fetch_resource::<Request<T>>(maybe_request, PhantomData).await)
}

pub async fn fetch_schema() -> StrResult<SchemaNode> {
    trace_err!(fetch_get("/api/settings-schema").await)
}

pub async fn fetch_session() -> StrResult<SessionDesc> {
    trace_err!(fetch_get("/api/session/load").await)
}

pub async fn apply_session_settings(settings: SessionSettings) -> StrResult {
    trace_err!(fetch_post("/api/session/store-settings", settings).await)
}

pub async fn apply_session_settings_raw(settings: String) -> StrResult {
    trace_err!(fetch_post("/api/session/store-settings", settings).await)
}
