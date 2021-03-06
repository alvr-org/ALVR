use crate::{
    sockets::{StreamId, LDC, LOCAL_IP},
    *,
};
use bytes::{Bytes, BytesMut};
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

pub type TcpStreamSendSocket = Arc<Mutex<SplitSink<Framed<TcpStream, LDC>, Bytes>>>;
pub type TcpStreamReceiveSocket = SplitStream<Framed<TcpStream, LDC>>;

pub async fn listen_for_server(port: u16) -> StrResult<TcpListener> {
    trace_err!(TcpListener::bind((LOCAL_IP, port)).await)
}

pub async fn accept_from_server(
    listener: TcpListener,
    server_ip: IpAddr,
) -> StrResult<(TcpStreamSendSocket, TcpStreamReceiveSocket)> {
    let (socket, server_address) = trace_err!(listener.accept().await)?;

    if server_address.ip() != server_ip {
        return fmt_e!(
            "Connected to wrong client: {} != {}",
            server_address,
            server_ip,
        );
    }

    let socket = Framed::new(socket, LDC::new());
    let (send_socket, receive_socket) = socket.split();

    Ok((Arc::new(Mutex::new(send_socket)), receive_socket))
}

pub async fn connect_to_client(
    client_ip: IpAddr,
    port: u16,
) -> StrResult<(TcpStreamSendSocket, TcpStreamReceiveSocket)> {
    let socket = trace_err!(TcpStream::connect((client_ip, port)).await)?;
    let socket = Framed::new(socket, LDC::new());
    let (send_socket, receive_socket) = socket.split();

    Ok((Arc::new(Mutex::new(send_socket)), receive_socket))
}

pub async fn receive_loop(
    mut socket: TcpStreamReceiveSocket,
    packet_enqueuers: Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<BytesMut>>>>,
) -> StrResult {
    while let Some(maybe_packet) = socket.next().await {
        let packet = trace_err!(maybe_packet)?;

        let stream_id = packet[0];
        if let Some(enqueuer) = packet_enqueuers.lock().await.get_mut(&stream_id) {
            trace_err!(enqueuer.send(packet))?;
        }
    }

    Ok(())
}
