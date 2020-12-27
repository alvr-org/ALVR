use super::*;
use crate::{data::*, logging::*, *};
use bytes::Bytes;
use futures::{
    prelude::*,
    stream::{SplitSink, SplitStream},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use tokio::{net::*, time};
use tokio_util::{codec::*, either::Either};

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);
const CONTROL_SOCKET_CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(500);

type ReceiverPart = SplitStream<Framed<TcpStream, LDC>>;
type SenderPart = SplitSink<Framed<TcpStream, LDC>, Bytes>;

async fn send<T: Serialize>(socket: &mut SenderPart, packet: &T) -> StrResult {
    let packet_bytes = trace_err!(bincode::serialize(packet))?;
    trace_err!(socket.send(packet_bytes.into()).await)
}

async fn recv<T: DeserializeOwned>(socket: &mut ReceiverPart) -> StrResult<T> {
    let packet_bytes = trace_err!(trace_none!(socket.next().await)?)?;
    trace_err!(bincode::deserialize(&packet_bytes))
}

pub struct ControlSocketSender<T> {
    sender: SenderPart,
    _phantom: PhantomData<T>,
}

impl<S: Serialize> ControlSocketSender<S> {
    fn new(sender: SenderPart) -> Self {
        Self {
            sender,
            _phantom: PhantomData,
        }
    }

    pub async fn send(&mut self, packet: &S) -> StrResult {
        send(&mut self.sender, packet).await
    }
}

pub struct ControlSocketReceiver<T> {
    receiver: ReceiverPart,
    _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    fn new(receiver: ReceiverPart) -> Self {
        Self {
            receiver,
            _phantom: PhantomData,
        }
    }

    pub async fn recv(&mut self) -> StrResult<R> {
        recv(&mut self.receiver).await
    }
}

pub enum ConnectionResult<S, R> {
    Connected {
        server_ip: IpAddr,
        control_sender: ControlSocketSender<S>,
        control_receiver: ControlSocketReceiver<R>,
        config_packet: ClientConfigPacket,
    },
    ServerMessage(ServerHandshakePacket),
}

async fn try_connect_to_server(
    handshake_socket: &mut UdpSocket,
    listener: &mut TcpListener,
    client_handshake_packet: &[u8],
    server_response_buffer: &mut [u8],
    headset_info: HeadsetInfoPacket,
) -> StrResult<Either<(IpAddr, SenderPart, ReceiverPart, ClientConfigPacket), ServerHandshakePacket>>
{
    trace_err!(
        handshake_socket
            .send_to(client_handshake_packet, (Ipv4Addr::BROADCAST, CONTROL_PORT))
            .await
    )?;

    let receive_response_loop = async move {
        loop {
            // this call will receive also the broadcasted client packet that must be ignored
            let (packet_size, _) =
                trace_err!(handshake_socket.recv_from(server_response_buffer).await)?;

            if let Ok(HandshakePacket::Server(handshake_packet)) =
                bincode::deserialize(&server_response_buffer[..packet_size])
            {
                break Ok(Either::Right(handshake_packet));
            }
        }
    };

    let (socket, server_address) = tokio::select! {
        maybe_pair = listener.accept() => trace_err!(maybe_pair)?,
        maybe_response = receive_response_loop => return maybe_response,
        _ = time::sleep(CLIENT_HANDSHAKE_RESEND_INTERVAL) => return trace_str!("timeout"),
    };

    let socket = Framed::new(socket, LDC::new());
    let (mut sender, mut receiver) = socket.split();

    send(&mut sender, &(headset_info, server_address.ip())).await?;

    let client_config = recv(&mut receiver).await?;

    Ok(Either::Left((
        server_address.ip(),
        sender,
        receiver,
        client_config,
    )))
}

// Return Some if server is compatible, otherwise return None
pub async fn connect_to_server<S: Serialize, R: DeserializeOwned>(
    headset_info: &HeadsetInfoPacket,
    device_name: String,
    hostname: String,
    certificate_pem: String,
) -> StrResult<ConnectionResult<S, R>> {
    let mut handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;
    trace_err!(handshake_socket.set_broadcast(true))?;

    let mut listener = trace_err!(TcpListener::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let client_handshake_packet = trace_err!(bincode::serialize(&HandshakePacket::Client(
        ClientHandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_CLIENT_VERSION.clone(),
            device_name,
            hostname,
            certificate_pem,
            reserved: "".into(),
        }
    )))?;

    let mut server_response_buffer = [0; MAX_HANDSHAKE_PACKET_SIZE_BYTES];
    loop {
        match try_connect_to_server(
            &mut handshake_socket,
            &mut listener,
            &client_handshake_packet,
            &mut server_response_buffer,
            headset_info.clone(),
        )
        .await
        {
            Ok(either) => match either {
                Either::Left((server_ip, sender, receiver, config_packet)) => {
                    break Ok(ConnectionResult::Connected {
                        server_ip,
                        control_sender: ControlSocketSender::new(sender),
                        control_receiver: ControlSocketReceiver::new(receiver),
                        config_packet,
                    })
                }
                Either::Right(server_packet) => {
                    break Ok(ConnectionResult::ServerMessage(server_packet))
                }
            },
            Err(e) => warn!("Error while connecting to server: {}", e),
        }
    }
}

pub struct PendingSocket {
    sender: SenderPart,
    receiver: ReceiverPart,
}

pub struct PendingClientConnection {
    pub pending_socket: PendingSocket,
    pub client_ip: IpAddr,
    pub server_ip: IpAddr,
    pub headset_info: HeadsetInfoPacket,
}

pub async fn begin_connecting_to_client(
    client_ips: &[IpAddr],
) -> StrResult<PendingClientConnection> {
    let client_addresses = client_ips
        .iter()
        .map(|&ip| (ip, CONTROL_PORT).into())
        .collect::<Vec<_>>();

    let socket = loop {
        match TcpStream::connect(client_addresses.as_slice()).await {
            Ok(socket) => break socket,
            Err(e) => {
                debug!("Timeout while connecting to clients: {}", e);
                tokio::time::sleep(CONTROL_SOCKET_CONNECT_RETRY_INTERVAL).await;
            }
        }
    };
    let client_ip = trace_err!(socket.peer_addr())?.ip();
    let socket = Framed::new(socket, LDC::new());
    let (sender, mut receiver) = socket.split();

    let (headset_info, server_ip) = recv(&mut receiver).await?;

    Ok(PendingClientConnection {
        pending_socket: PendingSocket { sender, receiver },
        client_ip,
        server_ip,
        headset_info,
    })
}

pub async fn finish_connecting_to_client<S: Serialize, R: DeserializeOwned>(
    pending_socket: PendingSocket,
    client_config: ClientConfigPacket,
) -> StrResult<(ControlSocketSender<S>, ControlSocketReceiver<R>)> {
    let PendingSocket {
        mut sender,
        receiver,
    } = pending_socket;

    send(&mut sender, &client_config).await?;

    Ok((
        ControlSocketSender::new(sender),
        ControlSocketReceiver::new(receiver),
    ))
}
