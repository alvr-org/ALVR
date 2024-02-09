use alvr_common::{anyhow::Result, glam::EulerRot};
use alvr_packets::BodyData;
use alvr_session::BodyTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{f32::consts::PI, ffi::CString, net::UdpSocket};

const RAD_TO_DEG: f32 = 180.0 / PI;

const BODY_JOINT_COUNT_FB: usize = 70;
const LOCATION_ORIENTATION_VALID: u64 = 0x00000001;
const LOCATION_POSITION_VALID: u64 = 0x00000002;
const LOCATION_ORIENTATION_TRACKED: u64 = 0x00000003;
const LOCATION_POSITION_TRACKED: u64 = 0x00000004;

pub const LOCATION_VALID: u64 = LOCATION_ORIENTATION_VALID | LOCATION_POSITION_VALID;
const LOCATION_TRACKED: u64 = LOCATION_ORIENTATION_TRACKED | LOCATION_POSITION_TRACKED;

pub struct BodyTrackingSink {
    config: BodyTrackingSinkConfig,
    socket: Option<UdpSocket>,
}

impl BodyTrackingSink {
    pub fn new(config: BodyTrackingSinkConfig, local_osc_port: u16) -> Result<Self> {
        match config {
            BodyTrackingSinkConfig::VrchatBodyOsc { port } => {
                let socket = UdpSocket::bind(format!("127.0.0.1:{local_osc_port}"))?;
                socket.connect(format!("127.0.0.1:{port}"))?;

                Ok(Self {
                    config,
                    socket: Some(socket),
                })
            },
            BodyTrackingSinkConfig::ViveTrackerProxy {} => {
                Ok(Self {
                    config,
                    socket: None,
                })
            },
        }
    }

    fn send_osc_message(&self, path: &str, args: Vec<OscType>) {
        if let Some(socket) = &self.socket {
            socket.send(
                &rosc::encoder::encode(&OscPacket::Message(OscMessage {
                    addr: path.into(),
                    args,
                }))
                .unwrap(),
            )
            .ok();
        }
    }

    pub fn send_tracking(&mut self, body_data: &BodyData) {
        match self.config {
            BodyTrackingSinkConfig::VrchatBodyOsc { .. } => {
                if let Some(poses) = &body_data.fb_body_skeleton {
                    if poses.len() == BODY_JOINT_COUNT_FB {
                        // XR_BODY_JOINT_HEAD_FB
                        let head = poses[7];
                        let head_euler = head.0.orientation.to_euler(EulerRot::ZXY);
                        if head.1 & LOCATION_VALID == LOCATION_VALID {
                            self.send_osc_message(
                                "/tracking/trackers/head/position",
                                vec![
                                    OscType::Float(-head.0.position.x),
                                    OscType::Float(head.0.position.y),
                                    OscType::Float(head.0.position.z),
                                ],
                            );
                            self.send_osc_message(
                                "/tracking/trackers/head/rotation",
                                vec![
                                    OscType::Float(head_euler.0 * RAD_TO_DEG),
                                    OscType::Float(head_euler.1 * RAD_TO_DEG),
                                    OscType::Float(head_euler.2 * RAD_TO_DEG),
                                ],
                            );
                        }
                        
                        // XR_BODY_JOINT_HIPS_FB 
                        let hips = poses[1];
                        let hips_euler = hips.0.orientation.to_euler(EulerRot::ZXY);
                        if hips.1 & LOCATION_VALID == LOCATION_VALID {
                            self.send_osc_message(
                                "/tracking/trackers/1/position",
                                vec![
                                    OscType::Float(-hips.0.position.x),
                                    OscType::Float(hips.0.position.y),
                                    OscType::Float(hips.0.position.z),
                                ],
                            );
                            self.send_osc_message(
                                "/tracking/trackers/1/rotation",
                                vec![
                                    OscType::Float(hips_euler.0 * RAD_TO_DEG),
                                    OscType::Float(hips_euler.1 * RAD_TO_DEG),
                                    OscType::Float(hips_euler.2 * RAD_TO_DEG),
                                ],
                            );
                        }
                        
                        // XR_BODY_JOINT_CHEST_FB  
                        let chest = poses[5];
                        let chest_euler = chest.0.orientation.to_euler(EulerRot::ZXY);
                        if chest.1 & LOCATION_VALID == LOCATION_VALID {
                            self.send_osc_message(
                                "/tracking/trackers/2/position",
                                vec![
                                    OscType::Float(-chest.0.position.x),
                                    OscType::Float(chest.0.position.y),
                                    OscType::Float(chest.0.position.z),
                                ],
                            );
                            self.send_osc_message(
                                "/tracking/trackers/2/rotation",
                                vec![
                                    OscType::Float(chest_euler.0 * RAD_TO_DEG),
                                    OscType::Float(chest_euler.1 * RAD_TO_DEG),
                                    OscType::Float(chest_euler.2 * RAD_TO_DEG),
                                ],
                            );
                        }
                        
                        // XR_BODY_JOINT_LEFT_ARM_LOWER_FB  
                        let left_elbow = poses[11];
                        let left_elbow_euler = left_elbow.0.orientation.to_euler(EulerRot::ZXY);
                        if left_elbow.1 & LOCATION_VALID == LOCATION_VALID {
                            self.send_osc_message(
                                "/tracking/trackers/3/position",
                                vec![
                                    OscType::Float(-left_elbow.0.position.x),
                                    OscType::Float(left_elbow.0.position.y),
                                    OscType::Float(left_elbow.0.position.z),
                                ],
                            );
                            self.send_osc_message(
                                "/tracking/trackers/3/rotation",
                                vec![
                                    OscType::Float(left_elbow_euler.0 * RAD_TO_DEG),
                                    OscType::Float(left_elbow_euler.1 * RAD_TO_DEG),
                                    OscType::Float(left_elbow_euler.2 * RAD_TO_DEG),
                                ],
                            );
                        }
                        
                        // XR_BODY_JOINT_RIGHT_ARM_LOWER_FB  
                        let right_elbow = poses[16];
                        let right_elbow_euler = right_elbow.0.orientation.to_euler(EulerRot::ZXY);
                        if right_elbow.1 & LOCATION_VALID == LOCATION_VALID {
                            self.send_osc_message(
                                "/tracking/trackers/4/position",
                                vec![
                                    OscType::Float(-right_elbow.0.position.x),
                                    OscType::Float(right_elbow.0.position.y),
                                    OscType::Float(right_elbow.0.position.z),
                                ],
                            );
                            self.send_osc_message(
                                "/tracking/trackers/4/rotation",
                                vec![
                                    OscType::Float(right_elbow_euler.0 * RAD_TO_DEG),
                                    OscType::Float(right_elbow_euler.1 * RAD_TO_DEG),
                                    OscType::Float(right_elbow_euler.2 * RAD_TO_DEG),
                                ],
                            );
                        }
                    }
                }
            }
            BodyTrackingSinkConfig::ViveTrackerProxy => { }
        }
    }
}
