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
use tokio_util::codec::*;

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);
const CONNECT_ERROR_RETRY_INTERVAL: Duration = Duration::from_millis(100);

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
    NetworkUnreachable,
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

    let listener = trace_err!(TcpListener::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let client_handshake_packet = trace_err!(bincode::serialize(&HandshakePacket::Client(
        ClientHandshakePacket {
            alvr_name: ALVR_NAME.into(),
            version: ALVR_VERSION.clone(),
            device_name,
            hostname,
            certificate_pem,
            reserved: "".into(),
        }
    )))?;

    let handshake_loop = async {
        loop {
            let broadcast_result = handshake_socket
                .send_to(
                    &client_handshake_packet,
                    (Ipv4Addr::BROADCAST, CONTROL_PORT),
                )
                .await;
            if broadcast_result.is_err() {
                return Ok(ConnectionResult::NetworkUnreachable);
            }

            let receive_response_loop = {
                let handshake_socket = &mut handshake_socket;
                async move {
                    let mut server_response_buffer = [0; MAX_HANDSHAKE_PACKET_SIZE_BYTES];
                    loop {
                        // this call will receive also the broadcasted client packet that must be ignored
                        let (packet_size, _) = trace_err!(
                            handshake_socket
                                .recv_from(&mut server_response_buffer)
                                .await
                        )?;

                        if let Ok(HandshakePacket::Server(handshake_packet)) =
                            bincode::deserialize(&server_response_buffer[..packet_size])
                        {
                            warn!("received packet {:?}", &handshake_packet);
                            break Ok(ConnectionResult::ServerMessage(handshake_packet));
                        }
                    }
                }
            };

            tokio::select! {
                res = receive_response_loop => break res,
                _ = time::sleep(CLIENT_HANDSHAKE_RESEND_INTERVAL) => {
                    warn!("Connection timeout, resending handhake packet");
                }
            }
        }
    };

    let try_connect_loop = async {
        loop {
            if let (Ok(pair), _) =
                tokio::join!(listener.accept(), time::sleep(CONNECT_ERROR_RETRY_INTERVAL))
            {
                break pair;
            }
        }
    };

    let (socket, server_address) = tokio::select! {
        res = handshake_loop => return res,
        pair = try_connect_loop => pair,
    };

    trace_err!(socket.set_nodelay(true))?;

    let socket = Framed::new(socket, LDC::new());
    let (mut sender, mut receiver) = socket.split();

    send(&mut sender, &(headset_info, server_address.ip())).await?;

    let config_packet = recv(&mut receiver).await?;

    Ok(ConnectionResult::Connected {
        server_ip: server_address.ip(),
        control_sender: ControlSocketSender::new(sender),
        control_receiver: ControlSocketReceiver::new(receiver),
        config_packet,
    })
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
        let res = tokio::join!(
            TcpStream::connect(client_addresses.as_slice()),
            time::sleep(CONNECT_ERROR_RETRY_INTERVAL)
        );
        match res {
            (Ok(socket), _) => break socket,
            (Err(e), _) => {
                debug!("Timeout while connecting to clients: {}", e);
            }
        }
    };

    trace_err!(socket.set_nodelay(true))?;

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
