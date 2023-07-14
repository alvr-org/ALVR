use super::{Ldc, CONTROL_PORT, LOCAL_IP};
use alvr_common::{anyhow::Result, ConResult, ToCon};
use bytes::Bytes;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, net::IpAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    time,
};
use tokio_util::codec::Framed;

pub struct ControlSocketSender<T> {
    inner: SplitSink<Framed<TcpStream, Ldc>, Bytes>,
    _phantom: PhantomData<T>,
}

impl<S: Serialize> ControlSocketSender<S> {
    pub fn send(&mut self, runtime: &Runtime, packet: &S) -> Result<()> {
        let packet_bytes = bincode::serialize(packet)?;
        runtime.block_on(self.inner.send(packet_bytes.into()))?;

        Ok(())
    }
}

pub struct ControlSocketReceiver<T> {
    inner: SplitStream<Framed<TcpStream, Ldc>>,
    _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    pub fn recv(&mut self, runtime: &Runtime, timeout: Duration) -> ConResult<R> {
        let packet_bytes = runtime.block_on(async {
            tokio::select! {
                res = self.inner.next() => res.map(|p| p.to_con()).to_con(),
                _ = time::sleep(timeout) => alvr_common::timeout(),
            }
        })??;
        bincode::deserialize(&packet_bytes).to_con()
    }
}

pub fn get_server_listener(runtime: &Runtime) -> Result<TcpListener> {
    let listener = runtime.block_on(TcpListener::bind((LOCAL_IP, CONTROL_PORT)))?;

    Ok(listener)
}

// Proto-control-socket that can send and receive any packet. After the split, only the packets of
// the specified types can be exchanged
pub struct ProtoControlSocket {
    inner: Framed<TcpStream, Ldc>,
}

pub enum PeerType<'a> {
    AnyClient(Vec<IpAddr>),
    Server(&'a TcpListener),
}

impl ProtoControlSocket {
    pub fn connect_to(
        runtime: &Runtime,
        timeout: Duration,
        peer: PeerType<'_>,
    ) -> ConResult<(Self, IpAddr)> {
        let socket = match peer {
            PeerType::AnyClient(ips) => {
                let client_addresses = ips
                    .iter()
                    .map(|&ip| (ip, CONTROL_PORT).into())
                    .collect::<Vec<_>>();
                runtime.block_on(async {
                    tokio::select! {
                        res = TcpStream::connect(client_addresses.as_slice()) => res.to_con(),
                        _ = time::sleep(timeout) => alvr_common::timeout(),
                    }
                })?
            }
            PeerType::Server(listener) => {
                let (socket, _) = runtime.block_on(async {
                    tokio::select! {
                        res = listener.accept() => res.to_con(),
                        _ = time::sleep(timeout) => alvr_common::timeout(),
                    }
                })?;
                socket
            }
        };

        socket.set_nodelay(true).to_con()?;
        let peer_ip = socket.peer_addr().to_con()?.ip();
        let socket = Framed::new(socket, Ldc::new());

        Ok((Self { inner: socket }, peer_ip))
    }

    pub fn send<S: Serialize>(&mut self, runtime: &Runtime, packet: &S) -> Result<()> {
        runtime.block_on(self.inner.send(bincode::serialize(packet)?.into()))?;

        Ok(())
    }

    pub fn recv<R: DeserializeOwned>(
        &mut self,
        runtime: &Runtime,
        timeout: Duration,
    ) -> ConResult<R> {
        let packet_bytes = runtime
            .block_on(async {
                tokio::select! {
                    res = self.inner.next() => res.map(|p| p.to_con()),
                    _ = time::sleep(timeout) => Some(alvr_common::timeout()),
                }
            })
            .to_con()??;

        bincode::deserialize(&packet_bytes).to_con()
    }

    pub fn split<S: Serialize, R: DeserializeOwned>(
        self,
    ) -> (ControlSocketSender<S>, ControlSocketReceiver<R>) {
        let (sender, receiver) = self.inner.split();

        (
            ControlSocketSender {
                inner: sender,
                _phantom: PhantomData,
            },
            ControlSocketReceiver {
                inner: receiver,
                _phantom: PhantomData,
            },
        )
    }
}
