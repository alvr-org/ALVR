use super::*;
use crate::{data::*, logging::*, *};
use futures::prelude::*;
use semver::Version;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use tokio::{net::*, time::timeout};
use tokio_util::codec::*;

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Serialize, Deserialize)]
enum HandshakeClientResponse {
    Ok {
        headset_info: HeadsetInfoPacket,
        server_ip: IpAddr,
    },
    IncompatibleServerVersion,
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
    ) -> StrResult<Option<(Self, ClientConfigPacket)>> {
        trace_err!(handshake_socket.send(client_handshake_packet).await)?;

        let (socket, server_address) = trace_err!(trace_err!(
            timeout(CLIENT_HANDSHAKE_RESEND_INTERVAL, listener.accept()).await
        )?)?;
        let mut socket = Framed::new(socket, LDC::new());

        let server_handshake_packet_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let server_version: Version =
            trace_err!(bincode::deserialize(&server_handshake_packet_bytes))?;

        if is_version_compatible(&server_version, &ALVR_SERVER_VERSION) {
            let response_bytes = trace_err!(bincode::serialize(&HandshakeClientResponse::Ok {
                headset_info,
                server_ip: server_address.ip(),
            }))?;
            trace_err!(socket.send(response_bytes.into()).await)?;
        } else {
            warn!("Server found with wrong version: {}", server_version);
            let response_bytes = trace_err!(bincode::serialize(
                &HandshakeClientResponse::IncompatibleServerVersion
            ))?;
            trace_err!(socket.send(response_bytes.into()).await)?;
            return Ok(None);
        }

        let client_config_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let client_config = trace_err!(bincode::deserialize(&client_config_bytes))?;

        Ok(Some((
            Self {
                peer_ip: server_address.ip(),
                socket,
                _phantom: PhantomData,
            },
            client_config,
        )))
    }

    // Return Some if server is compatible, otherwise return None
    pub async fn connect_to_server(
        device_name: String,
        headset_info: &HeadsetInfoPacket,
        hostname: String,
        certificate_pem: String,
    ) -> StrResult<Option<(Self, ClientConfigPacket)>> {
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
            identity: Some(PublicIdentity {
                hostname,
                certificate_pem
            }),
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
                Ok(maybe_pair) => break Ok(maybe_pair),
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

        let handshake_packet_bytes =
            trace_err!(bincode::serialize::<Version>(&ALVR_SERVER_VERSION))?;
        trace_err!(socket.send(handshake_packet_bytes.into()).await)?;

        let client_response_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let client_response = trace_err!(bincode::deserialize(&client_response_bytes))?;

        match client_response {
            HandshakeClientResponse::Ok {
                headset_info,
                server_ip,
            } => Ok(PendingClientConnection {
                pending_socket: PendingSocket { socket, peer_ip },
                server_ip,
                headset_info,
            }),
            HandshakeClientResponse::IncompatibleServerVersion => {
                trace_str!(id: LogId::IncompatibleServer)
            }
        }
    }

    pub async fn finish_connecting_to_client(
        pending_socket: PendingSocket,
        client_config: ClientConfigPacket,
    ) -> StrResult<Self> {
        let PendingSocket {
            mut socket,
            peer_ip,
        } = pending_socket;

        let client_config_bytes = trace_err!(bincode::serialize(&client_config))?;
        trace_err!(socket.send(client_config_bytes.into()).await)?;

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
    pub async fn send(&mut self, packet: S) -> StrResult {
        let packet_bytes = trace_err!(bincode::serialize(&packet))?;
        trace_err!(self.socket.send(packet_bytes.into()).await)
    }
}

impl<R: DeserializeOwned, S> ControlSocket<R, S> {
    pub async fn recv(&mut self) -> StrResult<R> {
        let packet_bytes = trace_err!(trace_none!(self.socket.next().await)?)?;
        trace_err!(bincode::deserialize(&packet_bytes))
    }
}
