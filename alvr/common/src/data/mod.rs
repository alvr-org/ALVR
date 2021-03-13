mod packets;
mod settings;
mod version;

#[cfg(not(feature = "new_dashboard"))]
mod legacy_session;
#[cfg(feature = "new_dashboard")]
mod session;

use crate::prelude::*;
use serde::{Deserialize, Serialize};

pub use packets::*;
pub use settings::*;
pub use version::*;

#[cfg(not(feature = "new_dashboard"))]
pub use legacy_session::*;
#[cfg(feature = "new_dashboard")]
pub use session::*;

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

pub fn create_identity(hostname: Option<String>) -> StrResult<PrivateIdentity> {
    let hostname = hostname.unwrap_or(format!("{}.client.alvr", rand::random::<u16>()));

    let certificate = trace_err!(rcgen::generate_simple_self_signed([hostname.clone()]))?;

    Ok(PrivateIdentity {
        hostname,
        certificate_pem: trace_err!(certificate.serialize_pem())?,
        key_pem: certificate.serialize_private_key_pem(),
    })
}
