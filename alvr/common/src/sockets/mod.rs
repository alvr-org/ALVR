mod control_socket;
mod stream_socket;

pub use control_socket::*;
pub use stream_socket::*;

use std::net::{IpAddr, Ipv4Addr};

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const CONTROL_PORT: u16 = 9943;
pub const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

type LDC = tokio_util::codec::LengthDelimitedCodec;
