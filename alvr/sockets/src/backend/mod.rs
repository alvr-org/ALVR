pub mod tcp;
pub mod udp;

use alvr_common::{anyhow::Result, ConResult};

pub trait SocketWriter: Send {
    fn send(&mut self, buffer: &[u8]) -> Result<()>;
}

// Trait used to abstract different socket (or other input/output) implementations. The funtionality
// is the intersection of the functionality of each implementation, that is it inheirits all
// limitations
pub trait SocketReader: Send {
    // Returns number of bytes written. buffer must be big enough to be able to receive a full
    // packet (size of MTU) otherwise data will be corrupted. The size of the data is
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize>;

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize>;
}
