use super::*;
use crate::{data::*, logging::*, *};
use futures::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};
use tokio::{net::*, time::timeout};
use tokio_util::codec::*;

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Serialize, Deserialize)]
enum HandshakeClientResponse {
    Ok {
        server_config: ServerConfigPacket,
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
        server_config: ServerConfigPacket,
    ) -> StrResult<Option<(Self, ClientConfigPacket)>> {
        trace_err!(handshake_socket.send(client_handshake_packet).await)?;

        let (socket, server_address) =
            match timeout(CLIENT_HANDSHAKE_RESEND_INTERVAL, listener.accept()).await {
                Ok(res) => trace_err!(res)?,
                Err(_) => {
                    debug!("Timeout while listening for server, retry");
                    return Ok(None);
                }
            };
        let mut socket = Framed::new(socket, LDC::new());

        let server_handshake_packet_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let server_handshake_packet: HandshakePacket =
            trace_err!(cbor::from_slice(&server_handshake_packet_bytes))?;

        let mut incompatible_server = false;

        if server_handshake_packet.alvr_name != ALVR_NAME {
            warn!("Received handshake packet: wrong name");
            incompatible_server = true;
        }

        match is_version_compatible(&server_handshake_packet.version, ALVR_SERVER_VERSION) {
            Ok(compatible) => {
                if !compatible {
                    warn!(
                        "Server found with wrong version: {}",
                        server_handshake_packet.version
                    );
                    incompatible_server = true;
                }
            }
            Err(e) => {
                warn!("Received handshake packet: {}", e);
                incompatible_server = true;
            }
        }

        if incompatible_server {
            let response_bytes = trace_err!(cbor::to_vec(
                &HandshakeClientResponse::IncompatibleServerVersion
            ))?;
            trace_err!(socket.send(response_bytes.into()).await)?;
            return Ok(None);
        } else {
            let response_bytes = trace_err!(cbor::to_vec(&HandshakeClientResponse::Ok {
                server_config: server_config.clone(),
                server_ip: server_address.ip(),
            }))?;
            trace_err!(socket.send(response_bytes.into()).await)?;
        }

        let client_config_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let client_config = trace_err!(cbor::from_slice(&client_config_bytes))?;

        Ok(Some((
            Self {
                peer_ip: server_address.ip(),
                socket,
                _phantom: PhantomData,
            },
            client_config,
        )))
    }

    pub async fn connect_to_server(
        server_config: ServerConfigPacket,
        hostname: String,
        certificate_pem: String,
    ) -> StrResult<(Self, ClientConfigPacket)> {
        let handshake_address = SocketAddr::V4(SocketAddrV4::new(MULTICAST_ADDR, CONTROL_PORT));

        let mut handshake_socket =
            trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;
        trace_err!(handshake_socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;
        trace_err!(handshake_socket.connect(handshake_address).await)?;

        let mut listener =
            trace_err!(TcpListener::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

        let client_handshake_packet = trace_err!(serde_cbor::to_vec(&HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_CLIENT_VERSION.into(),
            identity: Some(Identity {
                hostname,
                certificate_pem
            }),
        }))?;

        loop {
            match Self::try_connect_to_server(
                &mut handshake_socket,
                &mut listener,
                &client_handshake_packet,
                server_config.clone(),
            )
            .await
            {
                Ok(Some(pair)) => break Ok(pair),
                Err(e) => warn!("Error while connecting to server: {}", e),
                Ok(None) => (),
            }
        }
    }
}

impl ControlSocket<ClientControlPacket, ServerControlPacket> {
    pub async fn connect_to_client(
        client_ips: &[IpAddr],
        config_callback: impl FnOnce(ServerConfigPacket, IpAddr) -> ClientConfigPacket,
    ) -> StrResult<Self> {
        let client_addresses = client_ips
            .iter()
            .map(|&ip| SocketAddr::new(ip, CONTROL_PORT))
            .collect::<Vec<_>>();

        let socket = trace_err!(TcpStream::connect(client_addresses.as_slice()).await)?;
        let peer_ip = trace_err!(socket.peer_addr())?.ip();
        let mut socket = Framed::new(socket, LDC::new());

        let handshake_packet_bytes = trace_err!(cbor::to_vec(&HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_SERVER_VERSION.into(),
            identity: None,
        }))?;
        trace_err!(socket.send(handshake_packet_bytes.into()).await)?;

        let client_response_bytes = trace_err!(trace_none!(socket.next().await)?)?;
        let client_response = trace_err!(cbor::from_slice(&client_response_bytes))?;

        match client_response {
            HandshakeClientResponse::Ok {
                server_config,
                server_ip,
            } => {
                let client_config_bytes =
                    trace_err!(cbor::to_vec(&config_callback(server_config, server_ip)))?;
                trace_err!(socket.send(client_config_bytes.into()).await)?;

                Ok(Self {
                    peer_ip,
                    socket,
                    _phantom: PhantomData,
                })
            }
            HandshakeClientResponse::IncompatibleServerVersion => {
                trace_str!(id: LogId::IncompatibleServer)
            }
        }
    }
}

impl<R, S> ControlSocket<R, S> {
    pub fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }
}

impl<R, S: Serialize> ControlSocket<R, S> {
    pub async fn send(&mut self, packet: S) -> StrResult {
        let packet_bytes = trace_err!(cbor::to_vec(&packet))?;
        trace_err!(self.socket.send(packet_bytes.into()).await)
    }
}

impl<R: DeserializeOwned, S> ControlSocket<R, S> {
    pub async fn recv(&mut self) -> StrResult<R> {
        let packet_bytes = trace_err!(trace_none!(self.socket.next().await)?)?;
        trace_err!(cbor::from_slice(&packet_bytes))
    }
}
