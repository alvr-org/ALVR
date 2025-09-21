use alvr_common::{anyhow::Result, glam::EulerRot};
use alvr_packets::{FaceData, FaceExpressions};
use alvr_session::FaceTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{f32::consts::PI, net::UdpSocket};

const RAD_TO_DEG: f32 = 180.0 / PI;

const VRCFT_PORT: u16 = 0xA1F7;

pub struct FaceTrackingSink {
    config: FaceTrackingSinkConfig,
    socket: UdpSocket,
    packet_buffer: Vec<u8>,
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

    fn append_packet_vrcft(&mut self, prefix: [u8; 8], data: &[f32]) {
        self.packet_buffer.extend(prefix);

        for val in data {
            self.packet_buffer.extend(val.to_le_bytes());
        }
    }

    pub fn send_tracking(&mut self, face_data: &FaceData) {
        match self.config {
            FaceTrackingSinkConfig::VrchatEyeOsc { .. } => {
                if let [Some(left), Some(right)] = face_data.eyes_social {
                    let (left_pitch, left_yaw, _) = left.to_euler(EulerRot::XYZ);
                    let (right_pitch, right_yaw, _) = right.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/LeftRightPitchYaw",
                        vec![
                            OscType::Float(-left_pitch * RAD_TO_DEG),
                            OscType::Float(-left_yaw * RAD_TO_DEG),
                            OscType::Float(-right_pitch * RAD_TO_DEG),
                            OscType::Float(-right_yaw * RAD_TO_DEG),
                        ],
                    );
                } else if let Some(quat) = face_data.eyes_combined {
                    let (pitch, yaw, _) = quat.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/CenterPitchYaw",
                        vec![
                            OscType::Float(-pitch * RAD_TO_DEG),
                            OscType::Float(-yaw * RAD_TO_DEG),
                        ],
                    );
                }

                let (left_eye_blink, right_eye_blink) = match &face_data.face_expressions {
                    Some(FaceExpressions::Fb(items)) => (Some(items[12]), Some(items[13])),
                    Some(FaceExpressions::Pico(items)) => (Some(items[28]), Some(items[38])),
                    Some(FaceExpressions::Htc { eye, .. }) => {
                        (eye.as_ref().map(|v| v[0]), eye.as_ref().map(|v| v[2]))
                    }
                    _ => (None, None),
                };

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
            FaceTrackingSinkConfig::VrcFaceTracking => {
                self.packet_buffer.clear();

                if let [Some(left_quat), Some(right_quat)] = face_data.eyes_social {
                    let mut vec = left_quat.to_array().to_vec();
                    vec.extend_from_slice(&right_quat.to_array());
                    self.append_packet_vrcft(*b"EyesQuat", &vec);
                } else if let Some(quat) = face_data.eyes_combined {
                    self.append_packet_vrcft(*b"CombQuat", &quat.to_array());
                }

                match &face_data.face_expressions {
                    Some(FaceExpressions::Fb(items)) => {
                        self.append_packet_vrcft(*b"Face2Fb\0", items);
                    }
                    Some(FaceExpressions::Pico(items)) => {
                        self.append_packet_vrcft(*b"FacePico", items);
                    }
                    Some(FaceExpressions::Htc { eye, lip }) => {
                        if let Some(arr) = eye {
                            self.append_packet_vrcft(*b"EyesHtc\0", arr);
                        }

                        if let Some(arr) = lip {
                            self.append_packet_vrcft(*b"LipHtc\0\0", arr);
                        }
                    }
                    None => (),
                }

                self.socket.send(&self.packet_buffer).ok();
            }
        }
    }
}
