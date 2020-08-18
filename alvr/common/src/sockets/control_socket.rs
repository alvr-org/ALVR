use super::*;
use crate::{data::*, logging::*, *};
use futures::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};
use tokio::{net::*, time::timeout};
use tokio_serde::{
    formats::{Cbor, SymmetricalCbor},
    SymmetricallyFramed,
};
use tokio_util::codec::{self, FramedRead, FramedWrite, LengthDelimitedCodec};

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
    socket: tokio_serde::Framed<codec::Framed<TcpStream, LengthDelimitedCodec>, R, S, Cbor<R, S>>,
}

impl ControlSocket<ServerControlPacket, ClientControlPacket> {
    pub async fn connect_to_server(
        server_config: ServerConfigPacket,
        hostname: String,
        certificate_pem: String,
    ) -> StrResult<(Self, ClientConfigPacket)> {
        let handshake_address = SocketAddr::V4(SocketAddrV4::new(MULTICAST_ADDR, CONTROL_PORT));

        let mut handshake_socket =
            trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;
        trace_err!(handshake_socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;

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
            if let Err(e) = handshake_socket
                .send_to(&client_handshake_packet, handshake_address)
                .await
            {
                warn!("Error sending handshake packet: {}", e);
                continue;
            }

            let (mut socket, server_address) =
                match timeout(CLIENT_HANDSHAKE_RESEND_INTERVAL, listener.accept()).await {
                    Ok(res) => match res {
                        Ok(pair) => pair,
                        Err(e) => {
                            warn!("Failed to connect to server: {}", e);
                            continue;
                        }
                    },
                    Err(_) => {
                        debug!("Timeout while listening for server, retry");
                        continue;
                    }
                };

            let (receiver_socket, sender_socket) = socket.split();
            let receiver_socket = FramedRead::new(receiver_socket, LengthDelimitedCodec::new());
            let sender_socket = FramedWrite::new(sender_socket, LengthDelimitedCodec::new());

            let mut receiver_socket =
                SymmetricallyFramed::new(receiver_socket, SymmetricalCbor::default());
            let server_handshake_packet: HandshakePacket = match receiver_socket.next().await {
                Some(res) => match res {
                    Ok(packet) => packet,
                    Err(e) => {
                        warn!("Failed to deserialize handshake packet: {}", e);
                        continue;
                    }
                },
                None => {
                    warn!("Server disconnected while waiting for handshake packet");
                    continue;
                }
            };

            let mut incompatible_server = false;

            if server_handshake_packet.alvr_name != ALVR_NAME {
                warn!("Received handshake packet: wrong name");
                incompatible_server = true;
            }

            match is_version_compatible(&server_handshake_packet.version, ALVR_SERVER_VERSION_REQ) {
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

            let mut sender_socket =
                SymmetricallyFramed::new(sender_socket, SymmetricalCbor::default());
            if incompatible_server {
                if let Err(e) = sender_socket
                    .send(&HandshakeClientResponse::IncompatibleServerVersion)
                    .await
                {
                    warn!("Failed to send rejection packet: {}", e);
                }
                continue;
            } else if let Err(e) = sender_socket
                .send(&HandshakeClientResponse::Ok {
                    server_config: server_config.clone(),
                    server_ip: server_address.ip(),
                })
                .await
            {
                warn!("Failed to send config packet: {}", e);
                continue;
            }

            let mut receiver_socket =
                SymmetricallyFramed::new(receiver_socket.into_inner(), SymmetricalCbor::default());
            let client_config = match receiver_socket.next().await {
                Some(res) => match res {
                    Ok(config) => config,
                    Err(e) => {
                        warn!("Failed to deserialize config packet: {}", e);
                        continue;
                    }
                },
                None => {
                    warn!("Server disconnected while waiting for config packet");
                    continue;
                }
            };

            let socket = tokio_serde::Framed::new(
                codec::Framed::new(socket, LengthDelimitedCodec::new()),
                Cbor::default(),
            );

            break Ok((
                Self {
                    peer_ip: server_address.ip(),
                    socket,
                },
                client_config,
            ));
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

        let mut socket = trace_err!(
            TcpStream::connect(client_addresses.as_slice()).await,
            "Failed to connect to client"
        )?;

        let handshake_packet = HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_SERVER_VERSION.into(),
            identity: None,
        };

        let (receiver_socket, sender_socket) = socket.split();
        let receiver_socket = FramedRead::new(receiver_socket, LengthDelimitedCodec::new());
        let sender_socket = FramedWrite::new(sender_socket, LengthDelimitedCodec::new());

        let mut sender_socket = SymmetricallyFramed::new(sender_socket, SymmetricalCbor::default());
        trace_err!(
            sender_socket.send(handshake_packet).await,
            "Failed to send handshake packet"
        )?;

        let mut receiver_socket =
            SymmetricallyFramed::new(receiver_socket, SymmetricalCbor::default());

        let res = trace_none!(
            receiver_socket.next().await,
            "Client disconnected while waiting for response"
        )?;
        let client_response = trace_err!(res, "Failed to deserialize client response packet")?;

        match client_response {
            HandshakeClientResponse::Ok {
                server_config,
                server_ip,
            } => {
                let client_config = config_callback(server_config, server_ip);

                let mut sender_socket = SymmetricallyFramed::new(
                    sender_socket.into_inner(),
                    SymmetricalCbor::default(),
                );
                trace_err!(
                    sender_socket.send(client_config).await,
                    "Failed to send handshake packet"
                )?;

                Ok(Self {
                    peer_ip: trace_err!(socket.peer_addr())?.ip(),
                    socket: tokio_serde::Framed::new(
                        codec::Framed::new(socket, LengthDelimitedCodec::new()),
                        Cbor::default(),
                    ),
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

impl<R: DeserializeOwned + Unpin, S: Unpin> ControlSocket<R, S> {
    pub async fn recv(&mut self) -> StrResult<R> {
        match self.socket.next().await {
            Some(res) => trace_err!(res, "Error while receiving control packet"),
            None => trace_str!("Control socket: peer disconnected"),
        }
    }
}

impl<R: Unpin, S: Serialize + Unpin> ControlSocket<R, S> {
    pub async fn send(&mut self, packet: S) -> StrResult {
        trace_err!(
            self.socket.send(packet).await,
            "Error while sending control packet"
        )
    }
}
