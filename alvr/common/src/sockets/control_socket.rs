use super::{Ldc, CONTROL_PORT, LOCAL_IP};
use crate::prelude::*;
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
        let packet_bytes = trace_err!(bincode::serialize(packet))?;
        trace_err!(self.inner.send(packet_bytes.into()).await)
    }
}

pub struct ControlSocketReceiver<T> {
    inner: SplitStream<Framed<TcpStream, Ldc>>,
    _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    pub async fn recv(&mut self) -> StrResult<R> {
        let packet_bytes = trace_err!(trace_none!(self.inner.next().await)?)?;
        trace_err!(bincode::deserialize(&packet_bytes))
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
                trace_err!(TcpStream::connect(client_addresses.as_slice()).await)?
            }
            PeerType::Server => {
                let listener = trace_err!(TcpListener::bind((LOCAL_IP, CONTROL_PORT)).await)?;
                let (socket, _) = trace_err!(listener.accept().await)?;
                socket
            }
        };

        trace_err!(socket.set_nodelay(true))?;
        let peer_ip = trace_err!(socket.peer_addr())?.ip();
        let socket = Framed::new(socket, Ldc::new());

        Ok((Self { inner: socket }, peer_ip))
    }

    pub async fn send<S: Serialize>(&mut self, packet: &S) -> StrResult {
        let packet_bytes = trace_err!(bincode::serialize(packet))?;
        trace_err!(self.inner.send(packet_bytes.into()).await)
    }

    pub async fn recv<R: DeserializeOwned>(&mut self) -> StrResult<R> {
        let packet_bytes = trace_err!(trace_none!(self.inner.next().await)?)?;
        trace_err!(bincode::deserialize(&packet_bytes))
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
