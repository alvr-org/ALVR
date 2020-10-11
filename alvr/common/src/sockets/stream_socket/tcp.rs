use super::*;

fn create_socket(socket: TcpStream) -> StreamSocket {
    let socket = Framed::new(socket, LDC::new());
    let (send_stream, mut receive_stream) = socket.split();

    let packet_enqueuers = Arc::new(Mutex::new(HashMap::<_, mpsc::UnboundedSender<_>>::new()));

    let receive_loop = {
        let packet_enqueuers = packet_enqueuers.clone();
        async move {
            while let Some(maybe_packet) = receive_stream.next().await {
                let packet = trace_err!(maybe_packet)?;
                let mut packet_reader = packet.reader();
                let stream_id: StreamId =
                    trace_err!(bincode::deserialize_from(&mut packet_reader))?;

                if let Some(enqueuer) = packet_enqueuers.lock().await.get_mut(&stream_id) {
                    trace_err!(enqueuer.send(packet_reader.into_inner().freeze()))?;
                }
            }
            StrResult::Ok(())
        }
    };

    tokio::spawn(async move {
        if let Err(e) = receive_loop.await {
            warn!("Receive loop interrupted: {}", e);
        }
    });

    StreamSocket {
        socket_type: StreamSocketType::Tcp(Arc::new(Mutex::new(send_stream))),
        packet_enqueuers,
    }
}

pub(super) async fn connect_to_server(server_addr: SocketAddr) -> StrResult<StreamSocket> {
    let mut listener = trace_err!(TcpListener::bind((LOCAL_IP, server_addr.port())).await)?;
    let (socket, server_address) = trace_err!(listener.accept().await)?;

    if server_address.ip() != server_addr.ip() {
        return trace_str!("Connected to wrong client: {} != {}", server_address, server_addr);
    }

    Ok(create_socket(socket))
}

pub(super) async fn connect_to_client(client_addr: SocketAddr) -> StrResult<StreamSocket> {
    let socket = trace_err!(TcpStream::connect(client_addr).await)?;

    Ok(create_socket(socket))
}
