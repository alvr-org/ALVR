mod packets;
mod session;
mod settings;
mod version;

use serde::{Deserialize, Serialize};

pub use packets::*;
pub use version::*;

pub use session::*;
pub use settings::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicIdentity {
    pub hostname: String,
    pub certificate_pem: Option<String>,
}

pub struct PrivateIdentity {
    pub hostname: String,
    pub certificate_pem: String,
    pub key_pem: String,
}

use crate::prelude::*;

pub fn create_identity(hostname: Option<String>) -> StrResult<PrivateIdentity> {
    let hostname = hostname.unwrap_or(format!("{}.client.alvr", rand::random::<u16>()));

    let certificate = trace_err!(rcgen::generate_simple_self_signed([hostname.clone()]))?;

    Ok(PrivateIdentity {
        hostname,
        certificate_pem: trace_err!(certificate.serialize_pem())?,
        key_pem: certificate.serialize_private_key_pem(),
    })
}
