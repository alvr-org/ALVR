use super::*;

pub(super) async fn create_socket(peer_addr: SocketAddr) -> StrResult<StreamSocket> {
    let socket = trace_err!(UdpSocket::bind((LOCAL_IP, peer_addr.port())).await)?;
    let socket = UdpFramed::new(socket, LDC::new());
    let (send_stream, mut receive_stream) = socket.split();

    let packet_enqueuers = Arc::new(Mutex::new(HashMap::<_, mpsc::UnboundedSender<_>>::new()));

    let receive_loop = {
        let packet_enqueuers = packet_enqueuers.clone();
        async move {
            while let Some(maybe_packet) = receive_stream.next().await {
                let (packet, address) = trace_err!(maybe_packet)?;

                if address != peer_addr {
                    continue;
                }

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

    Ok(StreamSocket {
        socket_type: StreamSocketType::Udp {
            peer_addr,
            send_socket: Arc::new(Mutex::new(send_stream)),
        },
        packet_enqueuers,
    })
}
