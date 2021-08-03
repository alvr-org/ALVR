mod control_socket;
mod packets;
mod stream_socket;

pub use control_socket::*;
pub use packets::*;
pub use stream_socket::*;

use alvr_common::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const CONTROL_PORT: u16 = 9943;
pub const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

type Ldc = tokio_util::codec::LengthDelimitedCodec;

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
