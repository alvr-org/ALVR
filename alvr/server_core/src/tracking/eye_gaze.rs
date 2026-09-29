use alvr_common::{
    anyhow::Result,
    glam::{Quat, Vec3},
};
use rosc::{OscPacket, OscType};
use std::{
    io::ErrorKind,
    net::{Ipv4Addr, UdpSocket},
    time::{Duration, Instant},
};

const GAZE_ADDRESS: &str = "/alvr/eye/combined/v1";
const MAX_SAMPLE_AGE: Duration = Duration::from_millis(100);
const SEQUENCE_RESET_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_DATAGRAM_SIZE: usize = 256;
const MAX_PACKETS_PER_POLL: usize = 64;

pub struct EyeGazeReceiver {
    socket: UdpSocket,
    last_packet: Option<(u32, Instant)>,
    last_tracking_timestamp: Option<Duration>,
    last_polled_at: Instant,
    discard_queued: bool,
}

impl EyeGazeReceiver {
    pub fn new(port: u16) -> Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, port))?;
        socket.set_nonblocking(true)?;

        Ok(Self {
            socket,
            last_packet: None,
            last_tracking_timestamp: None,
            last_polled_at: Instant::now(),
            discard_queued: false,
        })
    }

    // Consume each observation once. Missing input must not refresh the foveation hold timer.
    pub fn receive(&mut self, tracking_timestamp: Duration) -> Result<Option<Quat>> {
        if self
            .last_tracking_timestamp
            .is_some_and(|previous| tracking_timestamp <= previous)
        {
            return Ok(None);
        }
        self.last_tracking_timestamp = Some(tracking_timestamp);

        let now = Instant::now();
        // After a tracking stall, drain the socket before accepting fresh observations again.
        self.discard_queued |= now.duration_since(self.last_polled_at) > MAX_SAMPLE_AGE;
        self.last_polled_at = now;
        let mut gaze = None;
        // One extra byte lets us reject oversized datagrams even if recv truncates them.
        let mut buffer = [0; MAX_DATAGRAM_SIZE + 1];

        for _ in 0..MAX_PACKETS_PER_POLL {
            let size = match self.socket.recv(&mut buffer) {
                Ok(size) => size,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    self.discard_queued = false;

                    return Ok(gaze);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                // Windows reports an oversized datagram instead of returning truncated bytes.
                Err(error) if error.raw_os_error() == Some(10040) => continue,
                Err(error) => return Err(error.into()),
            };
            if self.discard_queued || size > MAX_DATAGRAM_SIZE {
                continue;
            }
            let Some(packet) = buffer.get(..size) else {
                continue;
            };
            // This endpoint accepts single messages, not scheduled or recursive OSC bundles.
            if packet.first() != Some(&b'/') {
                continue;
            }
            let Ok((remaining, OscPacket::Message(message))) = rosc::decoder::decode_udp(packet)
            else {
                continue;
            };
            if !remaining.is_empty() || message.addr != GAZE_ADDRESS {
                continue;
            }
            let [
                OscType::Int(sequence),
                OscType::Int(valid),
                OscType::Int(age_us),
                OscType::Float(x),
                OscType::Float(y),
                OscType::Float(z),
                OscType::Float(w),
            ] = message.args.as_slice()
            else {
                continue;
            };
            if !matches!(valid, 0 | 1)
                || *age_us < 0
                || Duration::from_micros(*age_us as u64) > MAX_SAMPLE_AGE
            {
                continue;
            }

            let orientation = if *valid == 1 {
                let orientation = Quat::from_xyzw(*x, *y, *z, *w);
                let length_squared = orientation.length_squared();
                if !length_squared.is_finite()
                    || length_squared <= f32::EPSILON
                    || (orientation.normalize() * Vec3::NEG_Z).z >= -f32::EPSILON
                {
                    continue;
                }

                // Social output also consumes this quaternion and expects a unit rotation.
                Some(orientation.normalize())
            } else {
                None
            };
            // Interpret the OSC signed integer as the bit pattern of a wrapping u32 counter.
            let sequence = *sequence as u32;
            if let Some((previous, received_at)) = self.last_packet {
                let distance = sequence.wrapping_sub(previous);
                if distance == 0
                    || (now.duration_since(received_at) <= SEQUENCE_RESET_TIMEOUT
                        && distance >= 1 << 31)
                {
                    continue;
                }
            }

            self.last_packet = Some((sequence, now));
            gaze = orientation;
        }

        // Do not block head/controller tracking or use a potentially backlogged sample in a flood.
        self.discard_queued = true;

        Ok(None)
    }
}
