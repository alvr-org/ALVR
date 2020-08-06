use crate::{data::*, logging::*, *};
use async_bincode::*;
use futures::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};
use tokio::{net::*, time::timeout};

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 256;
const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

pub async fn search_client_loop<F: Future>(
    client_ip: Option<String>,
    client_found_cb: impl Fn(IpAddr) -> F,
) -> StrResult {
    let mut handshake_socket =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;
    trace_err!(handshake_socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;

    let maybe_target_client_ip = match client_ip {
        Some(ip_str) => Some(trace_err!(ip_str.parse::<IpAddr>(), "Client IP")?),
        None => None,
    };

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (hanshake_packet_size, address) =
            match handshake_socket.recv_from(&mut packet_buffer).await {
                Ok(pair) => pair,
                Err(e) => {
                    debug!("Error receiving handshake packet: {}", e);
                    continue;
                }
            };

        if let Some(ip) = maybe_target_client_ip {
            if address.ip() != ip {
                info!(id: LogId::ClientFoundWrongIp);
                continue;
            }
        }

        let handshake_packet: HandshakePacket =
            match bincode::deserialize(&packet_buffer[..hanshake_packet_size]) {
                Ok(client_handshake_packet) => client_handshake_packet,
                Err(e) => {
                    warn!(
                        id: LogId::ClientFoundInvalid,
                        "Received handshake packet: {}", e
                    );
                    continue;
                }
            };

        if handshake_packet.alvr_name != ALVR_NAME {
            warn!(
                id: LogId::ClientFoundInvalid,
                "Received handshake packet: wrong name"
            );
            continue;
        }

        match is_version_compatible(&handshake_packet.version, ALVR_CLIENT_VERSION_REQ) {
            Ok(compatible) => {
                if !compatible {
                    warn!(id: LogId::ClientFoundWrongVersion(handshake_packet.version));
                    continue;
                }
            }
            Err(e) => {
                warn!(
                    id: LogId::ClientFoundInvalid,
                    "Received handshake packet: {}", e
                );
                continue;
            }
        }

        client_found_cb(address.ip()).await;
    }
}

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
    socket: AsyncBincodeStream<TcpStream, R, S, AsyncDestination>,
}

impl ControlSocket<ServerControlPacket, ClientControlPacket> {
    pub async fn connect_to_server(
        server_config: ServerConfigPacket,
    ) -> StrResult<(Self, ClientConfigPacket)> {
        let handshake_address = SocketAddr::V4(SocketAddrV4::new(MULTICAST_ADDR, CONTROL_PORT));

        let mut handshake_socket =
            trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;
        trace_err!(handshake_socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;

        let mut listener =
            trace_err!(TcpListener::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

        let client_handshake_packet = trace_err!(bincode::serialize(&HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_CLIENT_VERSION.into(),
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

            let mut receiver_socket = AsyncBincodeReader::from(receiver_socket);
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

            let mut sender_socket = AsyncBincodeWriter::from(sender_socket).for_async();
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

            let mut receiver_socket = AsyncBincodeReader::from(receiver_socket.into_inner());
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

            break Ok((
                Self {
                    peer_ip: server_address.ip(),
                    socket: AsyncBincodeStream::from(socket).for_async(),
                },
                client_config,
            ));
        }
    }
}

impl ControlSocket<ClientControlPacket, ServerControlPacket> {
    pub async fn connect_to_client(
        client_ip: IpAddr,
        config_callback: impl FnOnce(ServerConfigPacket, IpAddr) -> ClientConfigPacket,
    ) -> StrResult<Self> {
        let client_address = SocketAddr::new(client_ip, CONTROL_PORT);

        let mut socket = trace_err!(
            TcpStream::connect(client_address).await,
            "Failed to connect to client"
        )?;

        let handshake_packet = HandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_SERVER_VERSION.into(),
        };

        let (receiver_socket, sender_socket) = socket.split();

        let mut sender_socket = AsyncBincodeWriter::from(sender_socket).for_async();
        trace_err!(
            sender_socket.send(handshake_packet).await,
            "Failed to send handshake packet"
        )?;

        let mut receiver_socket = AsyncBincodeReader::from(receiver_socket);

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

                let mut sender_socket =
                    AsyncBincodeWriter::from(sender_socket.into_inner()).for_async();
                trace_err!(
                    sender_socket.send(client_config).await,
                    "Failed to send handshake packet"
                )?;

                Ok(Self {
                    peer_ip: client_ip,
                    socket: AsyncBincodeStream::from(socket).for_async(),
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

impl<R: DeserializeOwned, S> ControlSocket<R, S> {
    pub async fn recv(&mut self) -> StrResult<R> {
        match self.socket.next().await {
            Some(res) => trace_err!(res, "Error while receiving control packet"),
            None => trace_str!("Control socket: peer disconnected"),
        }
    }
}

impl<R, S: Serialize> ControlSocket<R, S> {
    pub async fn send(&mut self, packet: S) -> StrResult {
        trace_err!(
            self.socket.send(packet).await,
            "Error wahile sending control packet"
        )
    }
}

enum StreamMode {
    Default,
    Reliable,
    PreferUnreliable,
}

trait StreamSocket {
    fn receive(&self);
    fn send_reliable(&self, stream_id: u8);
    fn send_unreliable(&self, stream_id: u8);
}

struct QuicSocket {}

// impl StreamSocket for QuicSocket {

// }

pub struct StreamManager {
    socket: Box<dyn StreamSocket>,
}

impl StreamManager {
    pub async fn new(peer_ip: IpAddr, port: u16, stream_socket_config: SocketConfig) -> StrResult<Self> {
        todo!()
    }
}
