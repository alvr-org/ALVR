use crate::{Ldc, LOCAL_IP};
use alvr_common::prelude::*;
use alvr_session::SocketBufferSize;
use bytes::{Buf, Bytes, BytesMut};
use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_util::codec::Framed;

pub type TcpStreamSendSocket = Arc<Mutex<SplitSink<Framed<TcpStream, Ldc>, Bytes>>>;
pub type TcpStreamReceiveSocket = SplitStream<Framed<TcpStream, Ldc>>;

pub fn set_socket_buffers(
    socket: &socket2::Socket,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> StrResult {
    info!(
        "Initial TCP buffer size: send: {}B, recv: {}B",
        socket.send_buffer_size().map_err(err!())?,
        socket.recv_buffer_size().map_err(err!())?
    );

    {
        let maybe_size = match send_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            socket.set_send_buffer_size(size as usize).map_err(err!())?;
            info!(
                "New TCP buffer send size: {}B",
                socket.send_buffer_size().map_err(err!())?
            );
        }
    }

    {
        let maybe_size = match recv_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            socket.set_recv_buffer_size(size as usize).map_err(err!())?;
            info!(
                "New TCP buffer recv size: {}B",
                socket.recv_buffer_size().map_err(err!())?
            );
        }
    }

    Ok(())
}

pub async fn bind(
    port: u16,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> StrResult<TcpListener> {
    let socket = TcpListener::bind((LOCAL_IP, port)).await.map_err(err!())?;
    let socket = socket2::Socket::from(socket.into_std().map_err(err!())?);

    set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes)?;

    TcpListener::from_std(socket.into()).map_err(err!())
}

pub async fn accept_from_server(
    listener: TcpListener,
    server_ip: IpAddr,
) -> StrResult<(TcpStreamSendSocket, TcpStreamReceiveSocket)> {
    let (socket, server_address) = listener.accept().await.map_err(err!())?;

    if server_address.ip() != server_ip {
        return fmt_e!("Connected to wrong client: {server_address} != {server_ip}");
    }

    socket.set_nodelay(true).map_err(err!())?;
    let socket = Framed::new(socket, Ldc::new());
    let (send_socket, receive_socket) = socket.split();

    Ok((Arc::new(Mutex::new(send_socket)), receive_socket))
}

pub async fn connect_to_client(
    client_ip: IpAddr,
    port: u16,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> StrResult<(TcpStreamSendSocket, TcpStreamReceiveSocket)> {
    let socket = TcpStream::connect((client_ip, port))
        .await
        .map_err(err!())?;
    let socket = socket2::Socket::from(socket.into_std().map_err(err!())?);

    set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes)?;

    let socket = TcpStream::from_std(socket.into()).map_err(err!())?;
    socket.set_nodelay(true).map_err(err!())?;
    let socket = Framed::new(socket, Ldc::new());
    let (send_socket, receive_socket) = socket.split();

    Ok((Arc::new(Mutex::new(send_socket)), receive_socket))
}

pub async fn receive_loop(
    mut socket: TcpStreamReceiveSocket,
    packet_enqueuers: Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<BytesMut>>>>,
) -> StrResult {
    while let Some(maybe_packet) = socket.next().await {
        let mut packet = maybe_packet.map_err(err!())?;

        let stream_id = packet.get_u16();
        if let Some(enqueuer) = packet_enqueuers.lock().await.get_mut(&stream_id) {
            enqueuer.send(packet).map_err(err!())?;
        }
    }

    Ok(())
}
