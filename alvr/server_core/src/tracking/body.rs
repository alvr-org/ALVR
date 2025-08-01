use alvr_common::{
    BODY_CHEST_ID, BODY_HIPS_ID, BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID,
    BODY_RIGHT_ELBOW_ID, BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, BodySkeleton, DeviceMotion,
    GENERIC_TRACKER_1_ID, GENERIC_TRACKER_2_ID, GENERIC_TRACKER_3_ID, HEAD_ID, anyhow::Result,
    glam::Vec3,
};
use alvr_session::BodyTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket, sync::LazyLock};

const CHEST_FB: usize = 5;
const HIPS_FB: usize = 1;
const LEFT_ARM_LOWER_FB: usize = 11;
const RIGHT_ARM_LOWER_FB: usize = 16;
const LEFT_LOWER_LEG_META: usize = 1;
const LEFT_FOOT_BALL_META: usize = 6;
const RIGHT_LOWER_LEG_META: usize = 8;
const RIGHT_FOOT_BALL_META: usize = 13;
const PELVIS_BD: usize = 0;
const LEFT_KNEE_BD: usize = 4;
const RIGHT_KNEE_BD: usize = 5;
const SPINE3_BD: usize = 9;
const LEFT_FOOT_BD: usize = 10;
const RIGHT_FOOT_BD: usize = 11;
const LEFT_ELBOW_BD: usize = 18;
const RIGHT_ELBOW_BD: usize = 19;

static BODY_TRACKER_OSC_PATH_MAP: LazyLock<HashMap<u64, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (*HEAD_ID, "/tracking/trackers/head/"),
        (*BODY_CHEST_ID, "/tracking/trackers/1/"),
        (*BODY_HIPS_ID, "/tracking/trackers/2/"),
        (*BODY_LEFT_ELBOW_ID, "/tracking/trackers/3/"),
        (*BODY_RIGHT_ELBOW_ID, "/tracking/trackers/4/"),
        (*BODY_LEFT_KNEE_ID, "/tracking/trackers/5/"),
        (*BODY_LEFT_FOOT_ID, "/tracking/trackers/6/"),
        (*BODY_RIGHT_KNEE_ID, "/tracking/trackers/7/"),
        (*BODY_RIGHT_FOOT_ID, "/tracking/trackers/8/"),
    ])
});

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
            }
            BodyTrackingSinkConfig::FakeViveTracker => Ok(Self {
                config,
                socket: None,
            }),
        }
    }

    fn send_osc_message(&self, path: &str, args: Vec<OscType>) {
        if let Some(socket) = &self.socket {
            socket
                .send(
                    &rosc::encoder::encode(&OscPacket::Message(OscMessage {
                        addr: path.into(),
                        args,
                    }))
                    .unwrap(),
                )
                .ok();
        }
    }

    pub fn send_tracking(&self, device_motions: &[(u64, DeviceMotion)]) {
        match self.config {
            BodyTrackingSinkConfig::VrchatBodyOsc { .. } => {
                for (id, motion) in device_motions {
                    if BODY_TRACKER_OSC_PATH_MAP.contains_key(id) {
                        // Only do position because rotation isn't quite right
                        let position = motion.pose.position;
                        self.send_osc_message(
                            format!(
                                "{}{}",
                                BODY_TRACKER_OSC_PATH_MAP.get(id).unwrap(),
                                "position"
                            )
                            .as_str(),
                            vec![
                                OscType::Float(position.x),
                                OscType::Float(position.y),
                                OscType::Float(-position.z),
                            ],
                        );
                    }
                }
            }
            BodyTrackingSinkConfig::FakeViveTracker => {}
        }
    }
}

// TODO: make this customizable
pub fn get_default_body_trackers_from_motion_trackers_bd(
    device_motions: &[(u64, DeviceMotion)],
) -> Vec<(u64, DeviceMotion)> {
    let mut poses = Vec::new();
    for (id, motion) in device_motions {
        if *id == *GENERIC_TRACKER_1_ID {
            poses.push((*BODY_HIPS_ID, *motion));
        } else if *id == *GENERIC_TRACKER_2_ID {
            poses.push((*BODY_LEFT_FOOT_ID, *motion));
        } else if *id == *GENERIC_TRACKER_3_ID {
            poses.push((*BODY_RIGHT_FOOT_ID, *motion));
        }
    }

    poses
}

// Obtain predefined joints as trackers
// TODO: make this customizable
pub fn extract_default_trackers(skeleton: &BodySkeleton) -> Vec<(u64, DeviceMotion)> {
    let mut poses = Vec::new();

    match skeleton {
        BodySkeleton::Fb(skeleton) => {
            if let Some(pose) = skeleton.upper_body[CHEST_FB] {
                poses.push((*BODY_CHEST_ID, pose));
            }

            if let Some(pose) = skeleton.upper_body[HIPS_FB] {
                poses.push((*BODY_HIPS_ID, pose));
            }

            if let Some(pose) = skeleton.upper_body[LEFT_ARM_LOWER_FB] {
                poses.push((*BODY_LEFT_ELBOW_ID, pose));
            }

            if let Some(pose) = skeleton.upper_body[RIGHT_ARM_LOWER_FB] {
                poses.push((*BODY_RIGHT_ELBOW_ID, pose));
            }

            if let Some(lower_body) = skeleton.lower_body {
                if let Some(pose) = lower_body[LEFT_LOWER_LEG_META] {
                    poses.push((*BODY_LEFT_KNEE_ID, pose));
                }

                if let Some(pose) = lower_body[LEFT_FOOT_BALL_META] {
                    poses.push((*BODY_LEFT_FOOT_ID, pose));
                }

                if let Some(pose) = lower_body[RIGHT_LOWER_LEG_META] {
                    poses.push((*BODY_RIGHT_KNEE_ID, pose));
                }

                if let Some(pose) = lower_body[RIGHT_FOOT_BALL_META] {
                    poses.push((*BODY_RIGHT_FOOT_ID, pose));
                }
            }
        }
        BodySkeleton::Bd(skeleton) => {
            if let Some(pose) = skeleton.0[SPINE3_BD] {
                poses.push((*BODY_HIPS_ID, pose));
            }

            if let Some(pose) = skeleton.0[PELVIS_BD] {
                poses.push((*BODY_CHEST_ID, pose));
            }

            if let Some(pose) = skeleton.0[LEFT_ELBOW_BD] {
                poses.push((*BODY_LEFT_ELBOW_ID, pose));
            }

            if let Some(pose) = skeleton.0[RIGHT_ELBOW_BD] {
                poses.push((*BODY_RIGHT_ELBOW_ID, pose));
            }

            if let Some(pose) = skeleton.0[LEFT_KNEE_BD] {
                poses.push((*BODY_LEFT_KNEE_ID, pose));
            }

            if let Some(pose) = skeleton.0[LEFT_FOOT_BD] {
                poses.push((*BODY_LEFT_FOOT_ID, pose));
            }

            if let Some(pose) = skeleton.0[RIGHT_KNEE_BD] {
                poses.push((*BODY_RIGHT_KNEE_ID, pose));
            }

            if let Some(pose) = skeleton.0[RIGHT_FOOT_BD] {
                poses.push((*BODY_RIGHT_FOOT_ID, pose));
            }
        }
    }

    poses
        .iter()
        .map(|(id, pose)| {
            (
                *id,
                DeviceMotion {
                    pose: *pose,
                    linear_velocity: Vec3::ZERO,
                    angular_velocity: Vec3::ZERO,
                },
            )
        })
        .collect()
}
