use super::*;
use crate::{data::*, logging::*, *};
use bytes::{buf::BufMutExt, Buf, BytesMut};
use quinn::*;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_serde::{formats::SymmetricalCbor, SymmetricallyFramed};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

type StreamId = super::StreamId;
type Certificate = quinn::Certificate;

#[derive(Serialize, Deserialize)]
struct StreamConfigPacket {
    stream_id: StreamId,
    reliable: bool,
}

pub enum QuickStreamSender {
    Reliable(
        SymmetricallyFramed<
            FramedWrite<SendStream, LengthDelimitedCodec>,
            BoxPacket,
            SymmetricalCbor<BoxPacket>,
        >,
    ),
    Unreliable {
        stream_id: StreamId,
        connection: Connection,
        packet_storage: BytesMut,
    },
}

#[async_trait]
impl StreamSender for QuickStreamSender {
    async fn send(&mut self, packet: BoxPacket) -> StrResult {
        match self {
            QuickStreamSender::Reliable(send_stream) => trace_err!(send_stream.send(packet).await),
            QuickStreamSender::Unreliable {
                stream_id,
                connection,
                packet_storage,
            } => {
                let id_packet = IdPacket {
                    id: *stream_id,
                    packet,
                };
                trace_err!(serde_cbor::to_writer(packet_storage.writer(), &id_packet))?;

                // consumes all bytes inside packet_storage
                trace_err!(connection.send_datagram(packet_storage.to_bytes()))
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
    unreliable_stream: Arc<Mutex<Datagrams>>,
    unpaired_receive_streams: Arc<Mutex<HashMap<StreamId, RecvStream>>>,
}

impl QuicStreamSocket {
    fn create_stream_socket(new_connection: NewConnection) -> Self {
        Self {
            connection: new_connection.connection,
            reliable_streams_listener: Arc::new(Mutex::new(new_connection.uni_streams)),
            unreliable_stream: Arc::new(Mutex::new(new_connection.datagrams)),
            unpaired_receive_streams: Arc::new(Mutex::new(HashMap::new())),
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
        let send_stream = trace_err!(self.connection.open_uni().await)?;

        let mut send_stream = SymmetricallyFramed::new(
            FramedWrite::new(send_stream, LengthDelimitedCodec::new()),
            SymmetricalCbor::default(),
        );

        let stream_config = StreamConfigPacket {
            stream_id,
            reliable: matches!(mode, StreamMode::PreferReliable),
        };
        trace_err!(send_stream.send(stream_config).await)?;

        match mode {
            StreamMode::PreferReliable => {
                let send_stream =
                    SymmetricallyFramed::new(send_stream.into_inner(), <_>::default());

                Ok(QuickStreamSender::Reliable(send_stream))
            }
            StreamMode::PreferUnreliable => Ok(QuickStreamSender::Unreliable {
                stream_id,
                connection: self.connection.clone(),
                packet_storage: BytesMut::new(),
            }),
        }
    }

    async fn subscribe_to_stream(&self, stream_type: StreamId) -> StrResult<QuickStreamReceiver> {
        todo!()
    }
}
