use alvr_common::{glam::EulerRot, prelude::*};
use alvr_session::FaceTrackingSinkConfig;
use alvr_sockets::{FaceData, LOCAL_IP};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};

const LOCAL_OSC_PORT: u16 = 9942;
const VRCHAT_PORT: u16 = 9000;
const VRC_FACE_TRACKING_PORT: u16 = 9999; // TBD

pub struct FaceTrackingSink {
    config: FaceTrackingSinkConfig,
    socket: UdpSocket,
}

impl FaceTrackingSink {
    pub fn new(config: FaceTrackingSinkConfig) -> StrResult<Self> {
        let port = match config {
            FaceTrackingSinkConfig::VrchatEyeOsc => VRCHAT_PORT,
            FaceTrackingSinkConfig::VrcFaceTrackingOsc => VRC_FACE_TRACKING_PORT,
        };

        let socket = UdpSocket::bind(SocketAddr::new(LOCAL_IP, LOCAL_OSC_PORT)).map_err(err!())?;
        socket
            .connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
            .map_err(err!())?;

        Ok(Self { config, socket })
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

    pub fn send_tracking(&self, face_data: FaceData) {
        match self.config {
            FaceTrackingSinkConfig::VrchatEyeOsc => {
                if let [Some(left), Some(right)] = face_data.eye_gazes {
                    let (left_pitch, left_yaw, _) = left.orientation.to_euler(EulerRot::XYZ);
                    let (right_pitch, right_yaw, _) = right.orientation.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/LeftRightPitchYaw",
                        vec![
                            OscType::Float(-left_pitch),
                            OscType::Float(-left_yaw),
                            OscType::Float(-right_pitch),
                            OscType::Float(-right_yaw),
                        ],
                    );
                } else if let Some(pose) = face_data.eye_gazes[0].or(face_data.eye_gazes[1]) {
                    let (pitch, yaw, _) = pose.orientation.to_euler(EulerRot::XYZ);

                    self.send_osc_message(
                        "/tracking/eye/CenterPitchYaw",
                        vec![OscType::Float(-pitch), OscType::Float(-yaw)],
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
            FaceTrackingSinkConfig::VrcFaceTrackingOsc => {
                if let Some(pose) = face_data.eye_gazes[0] {
                    self.send_osc_message(
                        "/tracking/eye/left/Quat",
                        vec![
                            OscType::Float(pose.orientation.w),
                            OscType::Float(pose.orientation.x),
                            OscType::Float(pose.orientation.y),
                            OscType::Float(pose.orientation.z),
                        ],
                    );
                } else {
                    self.send_osc_message("/tracking/eye/left/Active", vec![OscType::Bool(false)]);
                }
                if let Some(pose) = face_data.eye_gazes[1] {
                    self.send_osc_message(
                        "/tracking/eye/right/Quat",
                        vec![
                            OscType::Float(pose.orientation.w),
                            OscType::Float(pose.orientation.x),
                            OscType::Float(pose.orientation.y),
                            OscType::Float(pose.orientation.z),
                        ],
                    );
                } else {
                    self.send_osc_message("/tracking/eye/right/Active", vec![OscType::Bool(false)]);
                }

                if let Some(arr) = face_data.fb_face_expression {
                    self.send_osc_message(
                        "/tracking/face_fb",
                        arr.into_iter().map(OscType::Float).collect(),
                    );
                }

                if let Some(arr) = face_data.htc_eye_expression {
                    self.send_osc_message(
                        "/tracking/eye_htc",
                        arr.into_iter().map(OscType::Float).collect(),
                    );
                }
                if let Some(arr) = face_data.htc_lip_expression {
                    self.send_osc_message(
                        "/tracking/lip_htc",
                        arr.into_iter().map(OscType::Float).collect(),
                    );
                }
            }
        }
    }
}
