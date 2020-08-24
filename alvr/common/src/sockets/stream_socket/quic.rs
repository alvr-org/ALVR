use super::*;
use quinn::*;
use std::time::Duration;

type StreamId = super::StreamId;

pub(super) async fn request_stream(
    stream_id: StreamId,
    mode: StreamMode,
    connection: &Connection,
) -> StrResult<StreamSenderType> {
    // In case of unreliable stream, the reliable send_stream is used only to configure the stream
    let send_stream = trace_err!(connection.open_uni().await)?;
    let mut send_stream = FramedWrite::new(send_stream, LDC::new());

    let stream_config_bytes = trace_err!(bincode::serialize(&QuicStreamConfigPacket {
        stream_id,
        reliable: matches!(mode, StreamMode::PreferReliable),
    }))?;
    trace_err!(send_stream.send(stream_config_bytes.into()).await)?;

    Ok(match mode {
        StreamMode::PreferReliable => StreamSenderType::QuicReliable(send_stream),
        StreamMode::PreferUnreliable => StreamSenderType::QuicUnreliable(connection.clone()),
    })
}

pub(super) async fn subscribe_to_stream(
    stream_id: StreamId,
    reliable_streams_listener: &mut IncomingUniStreams,
    unpaired_stream_receivers: &mut HashMap<StreamId, StreamReceiverType>,
    unreliable_packet_enqueuers: Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<Bytes>>>>,
) -> StrResult<StreamReceiverType> {
    Ok(match unpaired_stream_receivers.remove(&stream_id) {
        Some(stream_receiver_type) => stream_receiver_type,
        None => loop {
            let receive_stream = trace_err!(trace_none!(reliable_streams_listener.next().await)?)?;
            let mut receive_stream = FramedRead::new(receive_stream, LDC::new());

            let stream_config_bytes = trace_err!(trace_none!(receive_stream.next().await)?)?;
            let stream_config: QuicStreamConfigPacket =
                trace_err!(bincode::deserialize(&stream_config_bytes))?;

            let stream_receiver = if stream_config.reliable {
                StreamReceiverType::QuicReliable(receive_stream)
            } else {
                let (enqueuer, dequeuer) = mpsc::unbounded_channel();
                unreliable_packet_enqueuers
                    .lock()
                    .await
                    .insert(stream_config.stream_id, enqueuer);
                StreamReceiverType::Dequeuer(dequeuer)
            };

            if stream_config.stream_id == stream_id {
                break stream_receiver;
            } else {
                unpaired_stream_receivers.insert(stream_config.stream_id, stream_receiver);
            }
        },
    })
}

fn create_socket(new_connection: NewConnection) -> StreamSocket {
    let mut unreliable_stream = new_connection.datagrams;
    let unreliable_packet_enqueuers =
        Arc::new(Mutex::new(HashMap::<_, mpsc::UnboundedSender<_>>::new()));

    let unreliable_receive_loop = {
        let unreliable_packet_enqueuers = unreliable_packet_enqueuers.clone();
        async move {
            while let Some(maybe_packet) = unreliable_stream.next().await {
                let packet = trace_err!(maybe_packet)?;
                let mut packet_reader = packet.reader();
                let stream_id: StreamId =
                    trace_err!(bincode::deserialize_from(&mut packet_reader))?;

                if let Some(enqueuer) = unreliable_packet_enqueuers.lock().await.get_mut(&stream_id)
                {
                    trace_err!(enqueuer.send(packet_reader.into_inner()))?;
                }
            }
            StrResult::Ok(())
        }
    };

    tokio::spawn(async move {
        if let Err(e) = unreliable_receive_loop.await {
            warn!("Unreliable receive loop interrupted: {}", e);
        }
    });

    StreamSocket {
        socket_type: StreamSocketType::Quic {
            connection: new_connection.connection,
            reliable_streams_listener: new_connection.uni_streams,
            unpaired_stream_receivers: HashMap::new(),
        },
        packet_enqueuers: unreliable_packet_enqueuers,
    }
}

pub(super) async fn connect_to_server(
    server_addr: SocketAddr,
    certificate_pem: String,
    key_pem: String,
    config: QuicConfig,
) -> StrResult<StreamSocket> {
    let mut transport_config = TransportConfig::default();
    if let Some(val) = config.stream_window_bidi {
        transport_config.stream_window_bidi(val);
    }
    if let Some(val) = config.stream_window_uni {
        transport_config.stream_window_uni(val);
    }
    if let Some(val) = config.max_idle_timeout_ms {
        trace_err!(transport_config.max_idle_timeout(val.into_option().map(Duration::from_millis)))?;
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

    let mut socket_config = ServerConfig::default();
    socket_config.transport = Arc::new(transport_config);

    let mut socket_config = ServerConfigBuilder::new(socket_config);

    if let Some(val) = config.use_stateless_retry {
        socket_config.use_stateless_retry(val);
    }

    let private_key = trace_err!(PrivateKey::from_pem(key_pem.as_bytes()))?;
    let cert_chain = trace_err!(CertificateChain::from_pem(certificate_pem.as_bytes()))?;
    trace_err!(socket_config.certificate(cert_chain, private_key))?;

    let socket_config = socket_config.build();
    debug!("QUIC socket config: {:?}", socket_config);

    let mut endpoint = Endpoint::builder();
    endpoint.listen(socket_config);

    let (_, mut incoming) =
        trace_err!(endpoint.bind(&SocketAddr::new(LOCAL_IP, server_addr.port())))?;

    let new_connection = trace_err!(trace_none!(incoming.next().await)?.await)?;

    if new_connection.connection.remote_address() != server_addr {
        return trace_str!("Connected to wrong client");
    }

    Ok(create_socket(new_connection))
}

pub(super) async fn connect_to_client(
    client_addr: SocketAddr,
    client_identity: Identity,
    config: QuicConfig,
) -> StrResult<StreamSocket> {
    let mut endpoint = Endpoint::builder();

    let mut socket_config = ClientConfigBuilder::default();
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

    let (endpoint, _) = trace_err!(endpoint.bind(&SocketAddr::new(LOCAL_IP, client_addr.port())))?;

    let new_connection =
        trace_err!(trace_err!(endpoint.connect(&client_addr, &client_identity.hostname))?.await)?;

    Ok(create_socket(new_connection))
}
