mod backend;
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

// Memory buffer that contains a hidden prefix
#[derive(Default)]
pub struct Buffer {
    inner: Vec<u8>,
    cursor: usize,
    length: usize,
}

impl Buffer {
    // Length of payload (without prefix)
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Note: this will not advance the cursor. Allocations are handled automatically
    // In case of reallocation, do not remove the cursor offset. This buffer is expected to be
    // reused and the total allocation size will not change after the running start.
    pub fn get_mut(&mut self, offset: usize, size: usize) -> &mut [u8] {
        let required_size = self.cursor + offset + size;
        if required_size > self.inner.len() {
            self.inner.resize(required_size, 0);
        }

        self.length = self.length.max(offset + size);

        &mut self.inner
    }

    pub fn get(&self) -> &[u8] {
        &self.inner[self.cursor..self.cursor + self.length]
    }

    pub fn advance_cursor(&mut self, count: usize) {
        self.cursor += count
    }

    // Clear buffer and cursor
    pub fn clear(&mut self) {
        self.cursor = 0;
        self.length = 0;
    }
}
