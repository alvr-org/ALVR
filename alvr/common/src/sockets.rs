// Note: for the StreamManager, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate

use crate::{data::*, logging::*, *};
use async_bincode::*;
use futures::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tokio::{net::*, time::timeout};

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 256;
const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

pub enum StreamType {
    Statistics, // used as a heartbeat
    Input,
    Haptics,
    PlayspaceSync,
    AudioStart,
    Audio,
    Video,
}

pub struct Certificate {
    hostname: String,
    certificate_pem: String,
    key_pem: String,
}

pub fn create_certificate(hostname: Option<String>) -> StrResult<Certificate> {
    let hostname = hostname.unwrap_or(format!("{}.client.alvr", rand::random::<u16>()));

    let certificate = trace_err!(rcgen::generate_simple_self_signed(vec![hostname.clone()]))?;

    Ok(Certificate {
        hostname,
        certificate_pem: trace_err!(certificate.serialize_pem())?,
        key_pem: certificate.serialize_private_key_pem(),
    })
}

fn handle_old_client(packet_bytes: &[u8]) {
    if packet_bytes.len() > 5 {
        if packet_bytes[..5] == *b"\x01ALVR" {
            warn!(id: LogId::ClientFoundWrongVersion("11 or previous".into()));
        } else if packet_bytes[..4] == *b"ALVR" {
            warn!(id: LogId::ClientFoundWrongVersion("12.x.x".into()));
        } else {
            debug!("Found unrelated packet during client discovery");
        }
    } else {
        debug!("Found unrelated packet during client discovery");
    }
}

pub async fn search_client_loop<F: Future>(
    client_found_cb: impl Fn(IpAddr, Identity) -> F,
) -> StrResult {
    let mut handshake_socket =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (handshake_packet_size, address) =
            match handshake_socket.recv_from(&mut packet_buffer).await {
                Ok(pair) => pair,
                Err(e) => {
                    debug!("Error receiving handshake packet: {}", e);
                    continue;
                }
            };

        if address.ip() != MULTICAST_ADDR {
            handle_old_client(&packet_buffer[..handshake_packet_size]);
            continue;
        }

        let handshake_packet: HandshakePacket =
            match bincode::deserialize(&packet_buffer[..handshake_packet_size]) {
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

        let identity = match handshake_packet.identity {
            Some(id) => id,
            None => {
                warn!(
                    id: LogId::ClientFoundInvalid,
                    "Received handshake packet: no identity",
                );
                continue;
            }
        };

        client_found_cb(address.ip(), identity).await;
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
        hostname: String,
        certificate_pem: String,
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
        client_ips: &[IpAddr],
        config_callback: impl FnOnce(ServerConfigPacket, IpAddr) -> ClientConfigPacket,
    ) -> StrResult<Self> {
        let client_addresses = client_ips
            .iter()
            .map(|&ip| SocketAddr::new(ip, CONTROL_PORT))
            .collect::<Vec<_>>();
        // let client_address = SocketAddr::new(client_ip, CONTROL_PORT);

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
                    peer_ip: trace_err!(socket.peer_addr())?.ip(),
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

struct QuicStreamSocket {
    connection: quinn::Connection,
}

impl QuicStreamSocket {
    async fn try_connect(peer_addr: SocketAddr, incoming: &mut quinn::Incoming) -> StrResult {
        let new_connection = trace_err!(trace_none!(incoming.next().await)?.await)?;

        if new_connection.connection.remote_address() != peer_addr {
            return trace_str!("Found wrong address");
        }

        Ok(())
    }

    // this method creates a "server socket" for the client to listen and connect to the server
    async fn connect_to_server(
        server_ip: IpAddr,
        port: u16,
        certificate_pem: String,
        key_pem: String,
        config: QuicConfig,
    ) -> StrResult<Self> {
        let mut transport_config = quinn::TransportConfig::default();
        if let Some(val) = config.stream_window_bidi {
            transport_config.stream_window_bidi(val);
        }
        if let Some(val) = config.stream_window_uni {
            transport_config.stream_window_uni(val);
        }
        if let Some(val) = config.max_idle_timeout_ms {
            trace_err!(
                transport_config.max_idle_timeout(val.into_option().map(Duration::from_millis))
            )?;
        }
        if let Some(val) = config.stream_receive_window {
            transport_config.stream_receive_window(val);
        }
        if let Some(val) = config.receive_window {
            transport_config.receive_window(val);
        }
        if let Some(val) = config.send_window {
            transport_config.send_window(val);
        }
        if let Some(val) = config.max_tlps {
            transport_config.max_tlps(val);
        }
        if let Some(val) = config.packet_threshold {
            transport_config.packet_threshold(val);
        }
        if let Some(val) = config.time_threshold {
            transport_config.time_threshold(val);
        }
        if let Some(val) = config.initial_rtt_ms {
            transport_config.initial_rtt(Duration::from_millis(val));
        }
        if let Some(val) = config.persistent_congestion_threshold {
            transport_config.persistent_congestion_threshold(val);
        }
        if let Some(val) = config.keep_alive_interval_ms {
            transport_config.keep_alive_interval(val.into_option().map(Duration::from_millis));
        }
        if let Some(val) = config.crypto_buffer_size {
            transport_config.crypto_buffer_size(val as _);
        }
        if let Some(val) = config.allow_spin {
            transport_config.allow_spin(val);
        }
        if let Some(val) = config.datagram_receive_buffer_size {
            transport_config.datagram_receive_buffer_size(val.into_option().map(|val| val as _));
        }
        if let Some(val) = config.datagram_send_buffer_size {
            transport_config.datagram_send_buffer_size(val as _);
        }

        let mut socket_config = quinn::ServerConfig::default();
        socket_config.transport = Arc::new(transport_config);

        let mut socket_config = quinn::ServerConfigBuilder::new(socket_config);

        if let Some(val) = config.use_stateless_retry {
            socket_config.use_stateless_retry(val);
        }

        let private_key = trace_err!(quinn::PrivateKey::from_pem(key_pem.as_bytes()))?;
        let cert_chain = trace_err!(quinn::CertificateChain::from_pem(
            certificate_pem.as_bytes()
        ))?;
        trace_err!(socket_config.certificate(cert_chain, private_key))?;

        let socket_config = socket_config.build();
        debug!("QUIC socket config: {:?}", socket_config);

        let mut endpoint = quinn::Endpoint::builder();
        endpoint.listen(socket_config);

        let (_, mut incoming) = trace_err!(endpoint.bind(&SocketAddr::new(LOCAL_IP, port)))?;

        let peer_addr = SocketAddr::new(server_ip, port);
        loop {
            if let Err(e) = QuicStreamSocket::try_connect(peer_addr, &mut incoming).await {
                warn!("Error while listening for server: {}", e);
            }
        }

        todo!()
    }

    // this method creates a "client socket" for the server to connect to the client
    async fn connect_to_client(
        client_ip: IpAddr,
        port: u16,
        client_identity: Identity,
        config: QuicConfig,
    ) -> StrResult<Self> {
        let mut endpoint = quinn::Endpoint::builder();

        let mut socket_config = quinn::ClientConfigBuilder::default();
        trace_err!(socket_config.add_certificate_authority(trace_err!(
            quinn::Certificate::from_pem(client_identity.certificate_pem.as_bytes())
        )?))?;
        if config.enable_0rtt {
            socket_config.enable_0rtt();
        }
        if config.enable_keylog {
            socket_config.enable_keylog();
        }
        // socket_config.protocols(...);

        let socket_config = socket_config.build();
        debug!("QUIC socket config: {:?}", socket_config);

        endpoint.default_client_config(socket_config);

        let (endpoint, _) = trace_err!(endpoint.bind(&SocketAddr::new(LOCAL_IP, port)))?;

        let new_connection = trace_err!(
            trace_err!(
                endpoint.connect(&SocketAddr::new(client_ip, port), &client_identity.hostname)
            )?
            .await
        )?;

        todo!()
    }
}

// impl StreamSocket for QuicSocket {

// }

pub struct StreamManager {
    socket: Box<dyn StreamSocket>,
}

impl StreamManager {
    pub async fn connect_to_client(
        peer_ip: IpAddr,
        port: u16,
        client_identity: Identity,
        stream_socket_config: SocketConfig,
    ) -> StrResult<Self> {
        todo!()
    }
}
