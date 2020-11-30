use super::*;
use crate::{data::*, logging::*, *};
use futures::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use tokio::{net::*, time::timeout};
use tokio_util::codec::*;

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

async fn send<T: Serialize>(socket: &mut Framed<TcpStream, LDC>, packet: &T) -> StrResult {
    let packet_bytes = trace_err!(bincode::serialize(packet))?;
    trace_err!(socket.send(packet_bytes.into()).await)
}

async fn recv<T: DeserializeOwned>(socket: &mut Framed<TcpStream, LDC>) -> StrResult<T> {
    let packet_bytes = trace_err!(trace_none!(socket.next().await)?)?;
    trace_err!(bincode::deserialize(&packet_bytes))
}

pub struct ControlSocket<R, S> {
    peer_ip: IpAddr,
    socket: Framed<TcpStream, LDC>,
    _phantom: PhantomData<(R, S)>,
}

impl ControlSocket<ServerControlPacket, ClientControlPacket> {
    async fn try_connect_to_server(
        handshake_socket: &mut UdpSocket,
        listener: &mut TcpListener,
        client_handshake_packet: &[u8],
        headset_info: HeadsetInfoPacket,
    ) -> StrResult<(Self, ClientConfigPacket)> {
        trace_err!(handshake_socket.send(client_handshake_packet).await)?;

        let (socket, server_address) = trace_err!(trace_err!(
            timeout(CLIENT_HANDSHAKE_RESEND_INTERVAL, listener.accept()).await
        )?)?;
        let mut socket = Framed::new(socket, LDC::new());

        send(&mut socket, &(headset_info, server_address.ip())).await?;

        let client_config = recv(&mut socket).await?;

        Ok((
            Self {
                peer_ip: server_address.ip(),
                socket,
                _phantom: PhantomData,
            },
            client_config,
        ))
    }

    // Return Some if server is compatible, otherwise return None
    pub async fn connect_to_server(
        headset_info: &HeadsetInfoPacket,
        device_name: String,
        hostname: String,
        certificate_pem: String,
    ) -> StrResult<(Self, ClientConfigPacket)> {
        let mut handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;
        trace_err!(handshake_socket.set_broadcast(true))?;
        trace_err!(
            handshake_socket
                .connect((Ipv4Addr::BROADCAST, CONTROL_PORT))
                .await
        )?;

        let mut listener = trace_err!(TcpListener::bind((LOCAL_IP, CONTROL_PORT)).await)?;

        let client_handshake_packet = trace_err!(bincode::serialize(&HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_CLIENT_VERSION.clone(),
            device_name,
            hostname,
            certificate_pem,
            reserved: "".into(),
        }))?;

        loop {
            match Self::try_connect_to_server(
                &mut handshake_socket,
                &mut listener,
                &client_handshake_packet,
                headset_info.clone(),
            )
            .await
            {
                Ok(pair) => break Ok(pair),
                Err(e) => warn!("Error while connecting to server: {}", e),
            }
        }
    }
}

pub struct PendingSocket {
    socket: Framed<TcpStream, LDC>,
    peer_ip: IpAddr,
}

pub struct PendingClientConnection {
    pub pending_socket: PendingSocket,
    pub server_ip: IpAddr,
    pub headset_info: HeadsetInfoPacket,
}

impl ControlSocket<ClientControlPacket, ServerControlPacket> {
    pub async fn begin_connecting_to_client(
        client_ips: &[IpAddr],
    ) -> StrResult<PendingClientConnection> {
        let client_addresses = client_ips
            .iter()
            .map(|&ip| (ip, CONTROL_PORT).into())
            .collect::<Vec<_>>();

        let socket = trace_err!(TcpStream::connect(client_addresses.as_slice()).await)?;
        let peer_ip = trace_err!(socket.peer_addr())?.ip();
        let mut socket = Framed::new(socket, LDC::new());

        let (headset_info, server_ip) = recv(&mut socket).await?;

        Ok(PendingClientConnection {
            pending_socket: PendingSocket { socket, peer_ip },
            server_ip,
            headset_info,
        })
    }

    pub async fn finish_connecting_to_client(
        pending_socket: PendingSocket,
        client_config: ClientConfigPacket,
    ) -> StrResult<Self> {
        let PendingSocket {
            mut socket,
            peer_ip,
        } = pending_socket;

        send(&mut socket, &client_config).await?;

        Ok(Self {
            peer_ip,
            socket,
            _phantom: PhantomData,
        })
    }
}

impl<R, S> ControlSocket<R, S> {
    pub fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }
}

impl<R, S: Serialize> ControlSocket<R, S> {
    pub async fn send(&mut self, packet: &S) -> StrResult {
        send(&mut self.socket, packet).await
    }
}

impl<R: DeserializeOwned, S> ControlSocket<R, S> {
    pub async fn recv(&mut self) -> StrResult<R> {
        recv(&mut self.socket).await
    }
}
