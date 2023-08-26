use alvr_common::{anyhow::Result, glam::EulerRot};
use alvr_packets::FaceData;
use alvr_session::FaceTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{f32::consts::PI, mem, net::UdpSocket};

const RAD_TO_DEG: f32 = 180.0 / PI;

const VRCFT_PORT: u16 = 0xA1F7;

pub struct FaceTrackingSink {
    config: FaceTrackingSinkConfig,
    socket: UdpSocket,
    packet_buffer: Vec<u8>,
    packet_cursor: usize,
}

impl FaceTrackingSink {
    pub fn new(config: FaceTrackingSinkConfig, local_osc_port: u16) -> Result<Self> {
        let port = match config {
            FaceTrackingSinkConfig::VrchatEyeOsc { port } => port,
            FaceTrackingSinkConfig::VrcFaceTracking => VRCFT_PORT,
        };

        let socket = UdpSocket::bind(format!("127.0.0.1:{local_osc_port}"))?;
        socket.connect(format!("127.0.0.1:{port}"))?;

        Ok(Self {
            config,
            socket,
            packet_buffer: vec![],
            packet_cursor: 0,
        })
    }

    fn send_osc_message(&self, path: &str, args: Vec<OscType>) {
        self.socket
            .send(
                &rosc::encoder::encode(&OscPacket::Message(OscMessage {
                    addr: path.into(),
                    args,
                }))
                .unwrap(),
            )
            .ok();
    }

    fn append_packet_vrcft(&mut self, prefix: &[u8; 8], data: &[f32]) {
        let new_buffer_len = self.packet_cursor + prefix.len() + data.len();
        if self.packet_buffer.len() < new_buffer_len {
            self.packet_buffer.resize(new_buffer_len, 0);
        }

        self.packet_buffer[self.packet_cursor..][..prefix.len()].copy_from_slice(prefix.as_slice());
        self.packet_cursor += prefix.len();

        for val in data {
            self.packet_buffer[self.packet_cursor..][..mem::size_of::<f32>()]
                .copy_from_slice(&val.to_le_bytes());
            self.packet_cursor += mem::size_of::<f32>();
        }
    }

    pub fn send_tracking(&mut self, face_data: FaceData) {
        match self.config {
            FaceTrackingSinkConfig::VrchatEyeOsc { .. } => {
                if let [Some(left), Some(right)] = face_data.eye_gazes {
                    let (left_pitch, left_yaw, _) = left.orientation.to_euler(EulerRot::XYZ);
                    let (right_pitch, right_yaw, _) = right.orientation.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/LeftRightPitchYaw",
                        vec![
                            OscType::Float(-left_pitch * RAD_TO_DEG),
                            OscType::Float(-left_yaw * RAD_TO_DEG),
                            OscType::Float(-right_pitch * RAD_TO_DEG),
                            OscType::Float(-right_yaw * RAD_TO_DEG),
                        ],
                    );
                } else if let Some(pose) = face_data.eye_gazes[0].or(face_data.eye_gazes[1]) {
                    let (pitch, yaw, _) = pose.orientation.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/CenterPitchYaw",
                        vec![
                            OscType::Float(-pitch * RAD_TO_DEG),
                            OscType::Float(-yaw * RAD_TO_DEG),
                        ],
                    );
                }

                let left_eye_blink = face_data
                    .fb_face_expression
                    .as_ref()
                    .map(|v| v[12])
                    .or_else(|| face_data.htc_eye_expression.as_ref().map(|v| v[0]));
                let right_eye_blink = face_data
                    .fb_face_expression
                    .map(|v| v[13])
                    .or_else(|| face_data.htc_eye_expression.map(|v| v[2]));

                if let (Some(left), Some(right)) = (left_eye_blink, right_eye_blink) {
                    self.send_osc_message(
                        "/tracking/eye/EyesClosedAmount",
                        vec![OscType::Float((left + right) / 2.0)],
                    );
                } else if let Some(blink) = left_eye_blink.or(right_eye_blink) {
                    self.send_osc_message(
                        "/tracking/eye/EyesClosedAmount",
                        vec![OscType::Float(blink)],
                    );
                }
            }
            FaceTrackingSinkConfig::VrcFaceTracking { .. } => {
                self.packet_cursor = 0;

                if let Some(arr) = face_data.fb_face_expression {
                    self.append_packet_vrcft(b"FaceFb\0\0", &arr);
                }

                if let Some(arr) = face_data.htc_eye_expression {
                    self.append_packet_vrcft(b"EyesHtc\0", &arr);
                }

                if let Some(arr) = face_data.htc_lip_expression {
                    self.append_packet_vrcft(b"LipHtc\0\0", &arr);
                }

                self.socket.send(&self.packet_buffer).ok();
            }
        }
    }
}
