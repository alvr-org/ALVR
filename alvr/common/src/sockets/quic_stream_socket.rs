use super::*;
use crate::{data::*, logging::*, *};
use bytes::{buf::BufExt, Buf, Bytes, BytesMut};
use quinn::*;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::FramedWrite;

type StreamId = super::StreamId;
type Certificate = quinn::Certificate;

#[derive(Serialize, Deserialize)]
struct StreamConfigPacket {
    stream_id: StreamId,
    reliable: bool,
}

pub enum QuickStreamSender {
    Reliable(FramedWrite<SendStream, LDC>),
    Unreliable {
        stream_id_bytes: Vec<u8>,
        connection: Connection,
    },
}

#[async_trait]
impl StreamSender for QuickStreamSender {
    async fn get_storage(&self) -> SendStorage {
        let prefix = if let QuickStreamSender::Unreliable {
            stream_id_bytes, ..
        } = self
        {
            stream_id_bytes.clone()
        } else {
            vec![]
        };

        SendStorage {
            prefix,
            buffer: BytesMut::new(),
        }
    }

    async fn send(&mut self, packet: &mut SendStorage) -> StrResult {
        match self {
            QuickStreamSender::Reliable(send_stream) => {
                trace_err!(send_stream.send(packet.buffer.to_bytes()).await)
            }
            QuickStreamSender::Unreliable { connection, .. } => {
                trace_err!(connection.send_datagram(packet.buffer.to_bytes()))
            }
        }
    }
}

pub enum QuickStreamReceiver {
    Reliable(),
    Unreliable {},
}

#[async_trait]
impl StreamReceiver for QuickStreamReceiver {
    async fn recv(&self) -> ReceivedPacket {
        todo!()
    }
}

pub struct QuicStreamSocket {
    connection: Connection,
    reliable_streams_listener: Arc<Mutex<IncomingUniStreams>>,
    unpaired_receive_streams: Arc<Mutex<HashMap<StreamId, RecvStream>>>,
    unreliable_packet_enqueuers: Arc<Mutex<HashMap<StreamId, mpsc::Sender<Bytes>>>>,
}

impl QuicStreamSocket {
    fn create_stream_socket(new_connection: NewConnection) -> Self {
        let mut unreliable_stream = new_connection.datagrams;
        let unreliable_packet_enqueuers =
            Arc::new(Mutex::new(HashMap::<StreamId, mpsc::Sender<Bytes>>::new()));

        let unreliable_receive_loop = {
            let unreliable_packet_enqueuers = unreliable_packet_enqueuers.clone();
            async move {
                while let Some(maybe_packet) = unreliable_stream.next().await {
                    let packet = trace_err!(maybe_packet)?;
                    let mut packet_reader = packet.reader();
                    let stream_id: StreamId = trace_err!(cbor::from_reader(&mut packet_reader))?;

                    if let Some(enqueuer) =
                        unreliable_packet_enqueuers.lock().await.get_mut(&stream_id)
                    {
                        trace_err!(enqueuer.send(packet_reader.into_inner()).await)?;
                    }
                }

                Ok::<_, String>(())
            }
        };

        tokio::spawn(async move {
            if let Err(e) = unreliable_receive_loop.await {
                warn!("Unreliable receive loop interrupted: {}", e);
            }
        });

        Self {
            connection: new_connection.connection,
            reliable_streams_listener: Arc::new(Mutex::new(new_connection.uni_streams)),
            // unreliable_stream: Arc::new(Mutex::new(new_connection.datagrams)),
            unpaired_receive_streams: Arc::new(Mutex::new(HashMap::new())),
            unreliable_packet_enqueuers,
        }
    }

    // this method creates a "server socket" for the client to listen and connect to the server
    pub async fn connect_to_server(
        server_addr: SocketAddr,
        certificate_pem: String,
        key_pem: String,
        config: QuicConfig,
    ) -> StrResult<Self> {
        let mut transport_config = TransportConfig::default();
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
            return trace_str!("Found wrong address");
        }

        Ok(Self::create_stream_socket(new_connection))
    }

    // this method creates a "client socket" for the server to connect to the client
    pub async fn connect_to_client(
        client_addr: SocketAddr,
        client_identity: Identity,
        config: QuicConfig,
    ) -> StrResult<Self> {
        let mut endpoint = Endpoint::builder();

        let mut socket_config = ClientConfigBuilder::default();
        trace_err!(
            socket_config.add_certificate_authority(trace_err!(Certificate::from_pem(
                client_identity.certificate_pem.as_bytes()
            ))?)
        )?;
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

        let (endpoint, _) =
            trace_err!(endpoint.bind(&SocketAddr::new(LOCAL_IP, client_addr.port())))?;

        let new_connection = trace_err!(
            trace_err!(endpoint.connect(&client_addr, &client_identity.hostname))?.await
        )?;

        Ok(Self::create_stream_socket(new_connection))
    }
}

#[async_trait]
impl StreamSocket for QuicStreamSocket {
    type Sender = QuickStreamSender;
    type Receiver = QuickStreamReceiver;

    async fn request_stream(
        &self,
        stream_id: StreamId,
        mode: StreamMode,
    ) -> StrResult<QuickStreamSender> {
        // in case of unreliable stream, use the reliable send_stream only to configure the stream
        let send_stream = trace_err!(self.connection.open_uni().await)?;
        let mut send_stream = FramedWrite::new(send_stream, LDC::new());

        let stream_config_bytes = trace_err!(cbor::to_vec(&StreamConfigPacket {
            stream_id,
            reliable: matches!(mode, StreamMode::PreferReliable),
        }))?;
        trace_err!(send_stream.send(stream_config_bytes.into()).await)?;

        match mode {
            StreamMode::PreferReliable => Ok(QuickStreamSender::Reliable(send_stream)),
            StreamMode::PreferUnreliable => Ok(QuickStreamSender::Unreliable {
                stream_id_bytes: trace_err!(cbor::to_vec(&stream_id))?,
                connection: self.connection.clone(),
            }),
        }
    }

    async fn subscribe_to_stream(&self, stream_type: StreamId) -> StrResult<QuickStreamReceiver> {
        // reliable_streams_listener.
        todo!()
    }
}
