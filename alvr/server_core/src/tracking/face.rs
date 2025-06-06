use alvr_common::{anyhow::Result, glam::EulerRot};
use alvr_packets::FaceData;
use alvr_session::FaceTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{f32::consts::PI, net::UdpSocket};

const RAD_TO_DEG: f32 = 180.0 / PI;

const VRCFT_PORT: u16 = 0xA1F7;

const FB_FACE_EXPRESSION_COUNT: usize = 70;
const PICO_FACE_EXPRESSION_COUNT: usize = 52;

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

    pub fn send_tracking(&mut self, face_data: FaceData) {
        // todo: introduce pico_face_expression field in FaceData
        let fb_face_expression = match &face_data.fb_face_expression {
            Some(face_expression) if face_expression.len() == FB_FACE_EXPRESSION_COUNT => {
                Some(face_expression)
            }
            _ => None,
        };

        let pico_face_expression = match &face_data.fb_face_expression {
            Some(face_expression) if face_expression.len() == PICO_FACE_EXPRESSION_COUNT => {
                Some(face_expression)
            }
            _ => None,
        };

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

                let left_eye_blink = fb_face_expression
                    .map(|v| v[12])
                    .or_else(|| face_data.htc_eye_expression.as_ref().map(|v| v[0]))
                    .or_else(|| pico_face_expression.map(|v| v[28]));
                let right_eye_blink = fb_face_expression
                    .map(|v| v[13])
                    .or_else(|| face_data.htc_eye_expression.as_ref().map(|v| v[2]))
                    .or_else(|| pico_face_expression.map(|v| v[38]));

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

                match face_data.eye_gazes {
                    [Some(left_quat), Some(right_quat)] => {
                        let mut vec = left_quat.orientation.to_array().to_vec();
                        vec.extend_from_slice(&right_quat.orientation.to_array());
                        self.append_packet_vrcft(*b"EyesQuat", &vec);
                    }
                    // todo: use separate field for combined eye data
                    [Some(combined_quat), None] => {
                        self.append_packet_vrcft(
                            *b"CombQuat",
                            &combined_quat.orientation.to_array(),
                        );
                    }
                    _ => (),
                }

                if let Some(arr) = fb_face_expression {
                    self.append_packet_vrcft(*b"Face2Fb\0", arr);
                }

                if let Some(arr) = pico_face_expression {
                    self.append_packet_vrcft(*b"FacePico", arr);
                }

                if let Some(arr) = face_data.htc_eye_expression {
                    self.append_packet_vrcft(*b"EyesHtc\0", &arr);
                }

                if let Some(arr) = face_data.htc_lip_expression {
                    self.append_packet_vrcft(*b"LipHtc\0\0", &arr);
                }

                self.socket.send(&self.packet_buffer).ok();
            }
        }
    }
}
