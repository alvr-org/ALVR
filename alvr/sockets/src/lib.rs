mod control_socket;
mod packets;
mod stream_socket;

use std::net::{IpAddr, Ipv4Addr};

pub use control_socket::*;
pub use packets::*;
pub use stream_socket::*;

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const CONTROL_PORT: u16 = 9943;
pub const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

type Ldc = tokio_util::codec::LengthDelimitedCodec;

mod util {
    use alvr_common::prelude::*;
    use std::future::Future;
    use tokio::{sync::oneshot, task};

    // Tokio tasks are not cancelable. This function awaits a cancelable task.
    pub async fn spawn_cancelable(
        future: impl Future<Output = StrResult> + Send + 'static,
    ) -> StrResult {
        // this channel is actually never used. cancel_receiver will be notified when _cancel_sender
        // is dropped
        let (_cancel_sender, cancel_receiver) = oneshot::channel::<()>();

        task::spawn(async {
            tokio::select! {
                res = future => res,
                _ = cancel_receiver => Ok(()),
            }
        })
        .await
        .map_err(err!())?
    }
}
pub use util::*;
