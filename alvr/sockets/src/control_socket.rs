use super::{Ldc, CONTROL_PORT, LOCAL_IP};
use alvr_common::prelude::*;
use bytes::Bytes;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, net::IpAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

pub struct ControlSocketSender<T> {
    inner: SplitSink<Framed<TcpStream, Ldc>, Bytes>,
    _phantom: PhantomData<T>,
}

impl<S: Serialize> ControlSocketSender<S> {
    pub async fn send(&mut self, packet: &S) -> StrResult {
        let packet_bytes = bincode::serialize(packet).map_err(err!())?;
        self.inner.send(packet_bytes.into()).await.map_err(err!())
    }
}

pub struct ControlSocketReceiver<T> {
    inner: SplitStream<Framed<TcpStream, Ldc>>,
    _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    pub async fn recv(&mut self) -> StrResult<R> {
        let packet_bytes = self
            .inner
            .next()
            .await
            .ok_or_else(enone!())?
            .map_err(err!())?;
        bincode::deserialize(&packet_bytes).map_err(err!())
    }
}

// Proto-control-socket that can send and receive any packet. After the split, only the packets of
// the specified types can be exchanged
pub struct ProtoControlSocket {
    inner: Framed<TcpStream, Ldc>,
}

pub enum PeerType {
    AnyClient(Vec<IpAddr>),
    Server,
}

impl ProtoControlSocket {
    pub async fn connect_to(peer: PeerType) -> StrResult<(Self, IpAddr)> {
        let socket = match peer {
            PeerType::AnyClient(ips) => {
                let client_addresses = ips
                    .iter()
                    .map(|&ip| (ip, CONTROL_PORT).into())
                    .collect::<Vec<_>>();
                TcpStream::connect(client_addresses.as_slice())
                    .await
                    .map_err(err!())?
            }
            PeerType::Server => {
                let listener = TcpListener::bind((LOCAL_IP, CONTROL_PORT))
                    .await
                    .map_err(err!())?;
                let (socket, _) = listener.accept().await.map_err(err!())?;
                socket
            }
        };

        socket.set_nodelay(true).map_err(err!())?;
        let peer_ip = socket.peer_addr().map_err(err!())?.ip();
        let socket = Framed::new(socket, Ldc::new());

        Ok((Self { inner: socket }, peer_ip))
    }

    pub async fn send<S: Serialize>(&mut self, packet: &S) -> StrResult {
        let packet_bytes = bincode::serialize(packet).map_err(err!())?;
        self.inner.send(packet_bytes.into()).await.map_err(err!())
    }

    pub async fn recv<R: DeserializeOwned>(&mut self) -> StrResult<R> {
        let packet_bytes = self
            .inner
            .next()
            .await
            .ok_or_else(enone!())?
            .map_err(err!())?;
        bincode::deserialize(&packet_bytes).map_err(err!())
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
