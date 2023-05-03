mod control_socket;
mod discovery;
mod packets;
mod stream;
mod stream_socket_old;
mod sharding;

use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

pub use control_socket::*;
use libp2p::{identity::Keypair, PeerId};
use libp2p_identity::PublicKey;
pub use packets::*;
pub use stream_socket_old::*;

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);

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

pub fn generate_identity_keys() -> Vec<u8> {
    Keypair::generate_ed25519().to_protobuf_encoding().unwrap()
}

pub fn identity_keys_to_public_key(keys: &[u8]) -> PublicKey {
    Keypair::from_protobuf_encoding(keys).unwrap().public()
}

pub fn public_key_to_string(key: PublicKey) -> String {
    key.to_peer_id().to_base58()
}