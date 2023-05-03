//! libp2p socket implementation. This is used for all streams if libp2p is chosen or just for control messages if UDP is preferred

use alvr_common::{prelude::*, StrResult, once_cell::sync::Lazy};
use futures::{channel::mpsc, future::Either, StreamExt};
use libp2p::{
    core::{muxing::StreamMuxerBox, upgrade},
    gossipsub::{self, MessageAuthenticity, TopicHash, IdentTopic},
    identity::Keypair,
    noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, Swarm, Transport,
};
use libp2p_quic as quic;
use std::{collections::HashMap, net::IpAddr};

// static 

// Only the control topic is guaranteed to be run with libp2p, other topics could be run with UDP.
static CONTROL_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("CC"));
static VIDEO_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("VV"));
static AUDIO_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("AA"));
static TRACKING_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("TT"));
static BUTTONS_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("BB"));
static HAPTICS_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("HH"));
static STATISTICS_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("SS"));
fn video_shard_topic(shard: usize) -> IdentTopic {
    IdentTopic::new(format!("V{shard}"))
}

static CONTROL_HASH: Lazy<IdentTopic> = Lazy::new(|| CONTROL_TOPIC.hash());
static VIDEO_HASH: Lazy<IdentTopic> = Lazy::new(|| VIDEO_TOPIC.hash());
static AUDIO_HASH: Lazy<IdentTopic> = Lazy::new(|| AUDIO_TOPIC.hash());
static TRACKING_HASH: Lazy<IdentTopic> = Lazy::new(|| TRACKING_TOPIC.hash());
static BUTTONS_HASH: Lazy<IdentTopic> = Lazy::new(|| BUTTONS_TOPIC.hash());
static HAPTICS_HASH: Lazy<IdentTopic> = Lazy::new(|| HAPTICS_TOPIC.hash());
static STATISTICS_HASH: Lazy<IdentTopic> = Lazy::new(|| STATISTICS_TOPIC.hash());

pub struct Libp2pSocket {
    swarm: Swarm<gossipsub::Behaviour>,
    send_channel_sender: mpsc::UnboundedSender<(TopicHash, Vec<u8>)>,
    send_channel_receiver: mpsc::UnboundedReceiver<(TopicHash, Vec<u8>)>,
    recv_channel_senders: HashMap<TopicHash, mpsc::UnboundedSender<Vec<u8>>>,
}

impl Libp2pSocket {
    // returns: (socket, control receiver)
    pub fn new(
        identity_keys: &[u8],
        authenticate: bool,
        port: u16,
    ) -> StrResult<(Self, mpsc::UnboundedReceiver<Vec<u8>>)> {
        let keypair = Keypair::from_protobuf_encoding(identity_keys).map_err(err!())?;
        let id = keypair.public().to_peer_id();

        let transport = if authenticate {
            quic::async_std::Transport::new(quic::Config::new(&keypair))
                .or_transport(
                    tcp::async_io::Transport::new(tcp::Config::default().nodelay(true))
                        .upgrade(upgrade::Version::V1Lazy)
                        .authenticate(noise::NoiseAuthenticated::xx(&keypair).map_err(err!())?)
                        .multiplex(yamux::YamuxConfig::default())
                        .timeout(std::time::Duration::from_secs(20)),
                )
                .map(|either_output, _| match either_output {
                    Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                    Either::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                })
                .boxed()
        } else {
            // tcp::async_io::Transport::new(tcp::Config::default().nodelay(true))
            //     .upgrade(upgrade::Version::V1Lazy)
            //     .authenticate(noise::NoiseAuthenticated::xx(&keypair)).
            //     // .map(|either_output, hjkh| if let  either_output {
            //     //     Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            //     // })
            //     .boxed()
            todo!()
        };

        let mut gossipsub = gossipsub::Behaviour::new(
            if authenticate {
                MessageAuthenticity::Signed(keypair)
            } else {
                MessageAuthenticity::Author(id)
            },
            gossipsub::ConfigBuilder::default().build().unwrap(),
        )
        .unwrap();

        // Note: some topics (streams) will not be used, according to the configuration acquired in
        // a later moment.
        gossipsub.subscribe(&CONTROL_TOPIC).unwrap();
        gossipsub.subscribe(&VIDEO_TOPIC).unwrap();
        gossipsub.subscribe(&AUDIO_TOPIC).unwrap();
        gossipsub.subscribe(&TRACKING_TOPIC).unwrap();
        gossipsub.subscribe(&BUTTONS_TOPIC).unwrap();
        gossipsub.subscribe(&HAPTICS_TOPIC).unwrap();
        gossipsub.subscribe(&STATISTICS_TOPIC).unwrap();

        let mut swarm = SwarmBuilder::with_async_std_executor(transport, gossipsub, id).build();

        swarm
            .listen_on(format!("/ip4/0.0.0.0/udp/{port}/quic-v1").parse().unwrap())
            .map_err(err!());
        swarm
            .listen_on(format!("/ip6/::1/udp/{port}/quic-v1").parse().unwrap())
            .map_err(err!());
        swarm
            .listen_on(format!("/ip4/0.0.0.0/tcp/{port}").parse().unwrap())
            .map_err(err!());
        swarm
            .listen_on(format!("/ip6/::1/tcp/{port}").parse().unwrap())
            .map_err(err!());

        let (outbound_sender, outbound_receiver) = mpsc::unbounded();

        let (inbound_control_sender, inbound_control_receiver) = mpsc::unbounded();

        Ok((
            Self {
                swarm,
                send_channel_sender: outbound_sender,
                send_channel_receiver: outbound_receiver,
                recv_channel_senders: [(&CONTROL_HASH, inbound_control_sender)]
                    .into_iter()
                    .collect(),
            },
            inbound_control_receiver,
        ))
    }

    pub fn connect_peer(&mut self, addr: IpAddr) {
        let multiaddr: Multiaddr = addr.into();
        self.swarm.dial(multiaddr).unwrap();
    }

    pub fn request_stream(&self) -> mpsc::UnboundedSender<(TopicHash, Vec<u8>)> {
        self.send_channel_sender.clone()
    }

    pub fn subscribe_to_stream(&mut self, topic: TopicHash) -> mpsc::UnboundedReceiver<Vec<u8>> {
        let (sender, receiver) = mpsc::unbounded();
        self.recv_channel_senders.insert(topic, sender);

        receiver
    }

    fn handle_event(&mut self, event: SwarmEvent<gossipsub::Event, gossipsub::HandlerError>) {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                } => {
                    if let Some(sender) = self.recv_channel_senders.get(&message.topic) {
                        sender.unbounded_send(message.data).ok()
                    }
                }
                gossipsub::Event::GossipsubNotSupported { peer_id } => error!("Protocol error"),
                _ => (),
            },
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => (),
            SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
            } => (),
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => (),
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
            } => (),
            SwarmEvent::OutgoingConnectionError { peer_id, error } => (),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => (),
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => (),
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => (),
            SwarmEvent::ListenerError { listener_id, error } => (),
            SwarmEvent::Dialing(_) => (),
            _ => (),
        }
    }

    pub async fn event_loop(&mut self) {
        loop {
            futures::select! {
                event = self.swarm.select_next_some() => self.handle_event(event),
                (topic, data) = self.send_channel_receiver.select_next_some() =>
                    self.swarm.behaviour_mut().publish(topic, data),
            };
        }
    }
}
