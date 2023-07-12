mod control_socket;
mod stream_socket;

use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

pub use control_socket::*;
pub use stream_socket::*;

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const CONTROL_PORT: u16 = 9943;
pub const HANDSHAKE_PACKET_SIZE_BYTES: usize = 56; // this may change in future protocols
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(500);

type Ldc = tokio_util::codec::LengthDelimitedCodec;
