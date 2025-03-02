mod backend;
mod control_socket;
mod stream_socket;

use alvr_common::{anyhow::Result, con_bail, info, AnyhowToCon, ConResult, ToCon};
use alvr_packets::{ClientControlPacket, ServerControlPacket};
use alvr_session::{DscpTos, SocketBufferSize, SocketProtocol};
use serde::{de::DeserializeOwned, Serialize};
use socket2::Socket;
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, TcpListener},
    time::Duration,
};

pub use control_socket::*;
pub use stream_socket::*;

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const CONTROL_PORT: u16 = 9943;
pub const HANDSHAKE_PACKET_SIZE_BYTES: usize = 56; // this may change in future protocols
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(500);
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(2);

pub const MDNS_SERVICE_TYPE: &str = "_alvr._tcp.local.";
pub const MDNS_PROTOCOL_KEY: &str = "protocol";
pub const MDNS_DEVICE_ID_KEY: &str = "device_id";

pub const WIRED_CLIENT_HOSTNAME: &str = "client.wired";

fn set_socket_buffers(
    socket: &socket2::Socket,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<()> {
    info!(
        "Initial socket buffer size: send: {}B, recv: {}B",
        socket.send_buffer_size()?,
        socket.recv_buffer_size()?
    );

    {
        let maybe_size = match send_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            if let Err(e) = socket.set_send_buffer_size(size as usize) {
                info!("Error setting socket send buffer: {e}");
            } else {
                info!(
                    "Set socket send buffer succeeded: {}",
                    socket.send_buffer_size()?
                );
            }
        }
    }

    {
        let maybe_size = match recv_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            if let Err(e) = socket.set_recv_buffer_size(size as usize) {
                info!("Error setting socket recv buffer: {e}");
            } else {
                info!(
                    "Set socket recv buffer succeeded: {}",
                    socket.recv_buffer_size()?
                );
            }
        }
    }

    Ok(())
}

fn set_dscp(socket: &Socket, dscp: Option<DscpTos>) {
    // https://en.wikipedia.org/wiki/Differentiated_services
    if let Some(dscp) = dscp {
        let tos = match dscp {
            DscpTos::BestEffort => 0,
            DscpTos::ClassSelector(precedence) => precedence << 3,
            DscpTos::AssuredForwarding {
                class,
                drop_probability,
            } => (class << 3) | drop_probability as u8,
            DscpTos::ExpeditedForwarding => 0b101110,
        };

        socket.set_tos((tos << 2) as u32).ok();
    }
}

// connect_to_client should be used on the server side.
// At the moment, the TcpListener is implemened on the client side so the API for this function
// is non-standard
// todo: convert to class when storing a TcpListener
pub fn connect_to_client<T: DeserializeOwned>(
    client_ips: Vec<IpAddr>,
    timeout: Duration,
) -> ConResult<(ProtoControlSocket, IpAddr, T)> {
    let (mut control_socket, client_ip) =
        ProtoControlSocket::connect_to(timeout, PeerType::AnyClient(client_ips))?;

    let res = control_socket.recv(timeout)?;

    Ok((control_socket, client_ip, res))
}

pub fn listen_to_server<T: DeserializeOwned>(
    listener_socket: &TcpListener,
    timeout: Duration,
    client_info: &impl Serialize,
) -> ConResult<(ProtoControlSocket, T)> {
    let (mut control_socket, _) =
        ProtoControlSocket::connect_to(timeout, PeerType::Server(listener_socket))?;

    control_socket.send(client_info).to_con()?;

    let config_packet = control_socket.recv(timeout)?;

    Ok((control_socket, config_packet))
}

pub fn send_restart_signal(
    mut control_socket: ProtoControlSocket,
    stream_config_packet: impl Serialize,
) -> ConResult<()> {
    // We must send the config packet before, which will be unused
    control_socket.send(&stream_config_packet).to_con()?;

    control_socket
        .send(&ServerControlPacket::Restarting)
        .to_con()
}

pub struct StreamSocketConfig {
    pub protocol: SocketProtocol,
    pub port: u16,
    pub send_buffer_bytes: SocketBufferSize,
    pub recv_buffer_bytes: SocketBufferSize,
    pub max_packet_size: usize,
    pub dscp: Option<DscpTos>,
}

pub enum ServerConnectionResult {
    Connected(SocketConnection),
    Restarting,
}

pub struct SocketConnection {
    control_socket: ProtoControlSocket,
    stream_socket: StreamSocket,
}

impl SocketConnection {
    // Note: the timeout resets after each internal operation
    pub fn from_client_connection(
        mut control_socket: ProtoControlSocket,
        timeout: Duration,
        stream_config_packet: impl Serialize,
        socket_config: StreamSocketConfig,
    ) -> ConResult<Self> {
        let client_ip = control_socket.inner.peer_addr().to_con()?.ip();

        control_socket.send(&stream_config_packet).to_con()?;

        control_socket
            .send(&ServerControlPacket::StartStream)
            .to_con()?;

        let signal = control_socket.recv(timeout)?;
        if !matches!(signal, ClientControlPacket::StreamReady) {
            con_bail!("Got unexpected packet waiting for stream ack");
        }

        let stream_socket = StreamSocketBuilder::connect_to_client(
            timeout,
            client_ip,
            socket_config.port,
            socket_config.protocol,
            socket_config.dscp,
            socket_config.send_buffer_bytes,
            socket_config.recv_buffer_bytes,
            socket_config.max_packet_size,
        )?;

        Ok(Self {
            control_socket,
            stream_socket,
        })
    }

    // Note: the timeout resets after each internal operation
    pub fn from_server_connection(
        mut control_socket: ProtoControlSocket,
        timeout: Duration,
        socket_config: StreamSocketConfig,
    ) -> ConResult<ServerConnectionResult> {
        let server_ip = control_socket.inner.peer_addr().to_con()?.ip();

        match control_socket.recv(timeout)? {
            ServerControlPacket::StartStream => (),
            ServerControlPacket::Restarting => return Ok(ServerConnectionResult::Restarting),
            _ => con_bail!("Got unexpected packet waiting for stream start"),
        }

        let stream_socket_builder = StreamSocketBuilder::listen_for_server(
            timeout,
            socket_config.port,
            socket_config.protocol,
            socket_config.dscp,
            socket_config.send_buffer_bytes,
            socket_config.recv_buffer_bytes,
        )
        .to_con()?;

        control_socket
            .send(&ClientControlPacket::StreamReady)
            .to_con()?;

        let stream_socket = stream_socket_builder.accept_from_server(
            server_ip,
            socket_config.port,
            socket_config.max_packet_size,
            timeout,
        )?;

        Ok(ServerConnectionResult::Connected(Self {
            control_socket,
            stream_socket,
        }))
    }

    pub fn request_reliable_stream<T>(&self) -> ConResult<ControlSocketSender<T>> {
        Ok(ControlSocketSender {
            inner: self.control_socket.inner.try_clone().to_con()?,
            buffer: vec![],
            _phantom: PhantomData,
        })
    }

    pub fn subscribe_to_reliable_stream<T>(&self) -> ConResult<ControlSocketReceiver<T>> {
        Ok(ControlSocketReceiver {
            inner: self.control_socket.inner.try_clone().to_con()?,
            buffer: vec![],
            recv_state: None,
            _phantom: PhantomData,
        })
    }

    pub fn request_unreliable_stream<T>(&self, stream_id: u16) -> StreamSender<T> {
        self.stream_socket.request_stream(stream_id)
    }

    pub fn subscribe_to_unreliable_stream<T>(
        &mut self,
        stream_id: u16,
        max_concurrent_buffers: usize,
    ) -> StreamReceiver<T> {
        self.stream_socket
            .subscribe_to_stream(stream_id, max_concurrent_buffers)
    }

    pub fn recv_poll(&mut self) -> ConResult<()> {
        self.stream_socket.recv()
    }
}
