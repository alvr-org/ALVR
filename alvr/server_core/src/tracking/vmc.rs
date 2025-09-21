use crate::tracking::HandType;
use alvr_common::{
    BODY_CHEST_ID, BODY_HIPS_ID, BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID,
    BODY_RIGHT_ELBOW_ID, BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, DeviceMotion, HAND_LEFT_ID,
    HAND_RIGHT_ID, HEAD_ID, Pose, anyhow::Result, glam::Quat,
};
use alvr_session::VMCConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket, sync::LazyLock};

// Transform DeviceMotion into Unity HumanBodyBones
// https://docs.unity3d.com/ScriptReference/HumanBodyBones.html
static DEVICE_MOTIONS_VMC_MAP: LazyLock<HashMap<u64, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (*HAND_LEFT_ID, "LeftHand"),
        (*HAND_RIGHT_ID, "RightHand"),
        (*BODY_CHEST_ID, "Chest"),
        (*BODY_HIPS_ID, "Hips"),
        (*BODY_LEFT_ELBOW_ID, "LeftLowerArm"),
        (*BODY_RIGHT_ELBOW_ID, "RightLowerArm"),
        (*BODY_LEFT_KNEE_ID, "LeftLowerLeg"),
        (*BODY_LEFT_FOOT_ID, "LeftFoot"),
        (*BODY_RIGHT_KNEE_ID, "RightLowerLeg"),
        (*BODY_RIGHT_FOOT_ID, "RightFoot"),
        (*HEAD_ID, "Head"),
    ])
});

#[expect(clippy::approx_constant)]
static DEVICE_MOTIONS_ROTATION_MAP: LazyLock<HashMap<u64, Quat>> = LazyLock::new(|| {
    HashMap::from([
        (
            *HAND_LEFT_ID,
            Quat::from_xyzw(-0.03538, 0.25483, -0.00000, -0.96634),
        ),
        (
            *HAND_RIGHT_ID,
            Quat::from_xyzw(-0.05859, -0.20524, -0.00000, 0.97696),
        ),
        (
            *BODY_CHEST_ID,
            Quat::from_xyzw(-0.49627, 0.49516, -0.43469, -0.56531),
        ),
        (
            *BODY_HIPS_ID,
            Quat::from_xyzw(-0.49274, 0.49568, -0.42416, -0.57584),
        ),
        (
            *BODY_RIGHT_ELBOW_ID,
            Quat::from_xyzw(-0.63465, -0.11567, 0.00000, 0.76410),
        ),
        (
            *BODY_LEFT_KNEE_ID,
            Quat::from_xyzw(0.51049, 0.47862, 0.42815, -0.57185),
        ),
        (
            *BODY_LEFT_FOOT_ID,
            Quat::from_xyzw(-0.59103, 0.38818, 0.00000, -0.70711),
        ),
        (
            *BODY_RIGHT_KNEE_ID,
            Quat::from_xyzw(-0.52823, 0.45434, -0.58530, -0.41470),
        ),
        (
            *BODY_RIGHT_FOOT_ID,
            Quat::from_xyzw(0.70228, -0.08246, 0.70711, 0.00000),
        ),
    ])
});

static HAND_SKELETON_VMC_MAP: [[(usize, &str); 1]; 2] = [[(0, "LeftHand")], [(0, "RightHand")]];

static HAND_SKELETON_ROTATIONS: LazyLock<[HashMap<usize, Quat>; 2]> = LazyLock::new(|| {
    [
        HashMap::from([(0, Quat::from_xyzw(-0.03566, 0.25481, 0.00000, -0.96633))]),
        HashMap::from([(0, Quat::from_xyzw(-0.05880, -0.20574, -0.00000, 0.97684))]),
    ]
});

pub struct VMCSink {
    socket: Option<UdpSocket>,
}

impl VMCSink {
    pub fn new(config: VMCConfig) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(format!("{}:{}", config.host, config.port))?;

        Ok(Self {
            socket: Some(socket),
        })
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

    pub fn send_hand_tracking(
        &self,
        hand_type: HandType,
        skeleton: &[Pose; 26],
        orientation_correction: bool,
    ) {
        let hand_id = hand_type as usize;
        for (part, vmc_str) in HAND_SKELETON_VMC_MAP[hand_id] {
            let corrected_orientation = {
                let mut q = skeleton[part].orientation;
                if orientation_correction {
                    if HAND_SKELETON_ROTATIONS[hand_id].contains_key(&part) {
                        q *= *HAND_SKELETON_ROTATIONS[hand_id].get(&part).unwrap();
                    }
                    q.z = -q.z;
                    q.w = -q.w;
                }
                q
            };

            self.send_osc_message(
                "/VMC/Ext/Bone/Pos",
                vec![
                    OscType::String(vmc_str.to_string()),
                    OscType::Float(skeleton[part].position.x),
                    OscType::Float(skeleton[part].position.y),
                    OscType::Float(skeleton[part].position.z),
                    OscType::Float(corrected_orientation.x),
                    OscType::Float(corrected_orientation.y),
                    OscType::Float(corrected_orientation.z),
                    OscType::Float(corrected_orientation.w),
                ],
            );
        }
    }

    pub fn send_tracking(
        &self,
        device_motions: &[(u64, DeviceMotion)],
        orientation_correction: bool,
    ) {
        for (id, motion) in device_motions {
            if DEVICE_MOTIONS_VMC_MAP.contains_key(id) {
                let corrected_orientation = {
                    let mut q = motion.pose.orientation;
                    if orientation_correction {
                        if DEVICE_MOTIONS_ROTATION_MAP.contains_key(id) {
                            q *= *DEVICE_MOTIONS_ROTATION_MAP.get(id).unwrap();
                        }
                        q.z = -q.z;
                        q.w = -q.w;
                    }
                    q
                };

                self.send_osc_message(
                    "/VMC/Ext/Bone/Pos",
                    vec![
                        OscType::String((*DEVICE_MOTIONS_VMC_MAP.get(id).unwrap()).to_string()),
                        OscType::Float(motion.pose.position.x),
                        OscType::Float(motion.pose.position.y),
                        OscType::Float(motion.pose.position.z),
                        OscType::Float(corrected_orientation.x),
                        OscType::Float(corrected_orientation.y),
                        OscType::Float(corrected_orientation.z),
                        OscType::Float(corrected_orientation.w),
                    ],
                );
            }
        }
    }
}
