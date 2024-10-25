use alvr_common::{
    anyhow::Result, glam::Quat, once_cell::sync::Lazy, DeviceMotion, Pose, BODY_CHEST_ID,
    BODY_HIPS_ID, BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID,
    BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket};

use alvr_session::VMCConfig;

pub use crate::tracking::HandType;

// Transform DeviceMotion into Unity HumanBodyBones
// https://docs.unity3d.com/ScriptReference/HumanBodyBones.html
static DEVICE_MOTIONS_VMC_MAP: Lazy<HashMap<u64, &'static str>> = Lazy::new(|| {
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

static DEVICE_MOTIONS_ROTATION_MAP: Lazy<HashMap<u64, Quat>> = Lazy::new(|| {
    HashMap::from([
        (
            *HAND_LEFT_ID,
            Quat::from_xyzw(
                -6.213430570750633e-08,
                -1.7979416426202113e-07,
                0.7071067411992601,
                0.7071065897770331,
            ),
        ),
        (
            *HAND_RIGHT_ID,
            Quat::from_xyzw(
                0.3219087189747184,
                0.7288832784221684,
                0.3392694392278636,
                -0.4999999512887856,
            ),
        ),
        (
            *BODY_CHEST_ID,
            Quat::from_xyzw(
                -0.48548752779498533,
                0.5137675867233772,
                -0.5131288074683715,
                -0.4868713724975375,
            ),
        ),
        (
            *BODY_HIPS_ID,
            Quat::from_xyzw(
                -0.38920734358587283,
                0.4731761921254787,
                -0.25037835650145845,
                -0.7496216673275858,
            ),
        ),
        (
            *BODY_LEFT_KNEE_ID,
            Quat::from_xyzw(
                0.42592041950463216,
                0.5417784481730453,
                0.3880448422573891,
                -0.6119550893141193,
            ),
        ),
        (
            *BODY_LEFT_FOOT_ID,
            Quat::from_xyzw(
                -0.15405857492497116,
                0.6901198926998461,
                -2.2748115460768936e-08,
                -0.7071068140641757,
            ),
        ),
        (
            *BODY_RIGHT_KNEE_ID,
            Quat::from_xyzw(
                -0.4650211913041333,
                0.5058610693118244,
                -0.6180250915435218,
                -0.381974687482609,
            ),
        ),
        (
            *BODY_RIGHT_FOOT_ID,
            Quat::from_xyzw(
                0.6815561983146281,
                -0.18836473838436454,
                0.7071065841771154,
                5.6213965690665724e-08,
            ),
        ),
    ])
});

static HAND_SKELETON_VMC_MAP: Lazy<[[(usize, &'static str); 1]; 2]> =
    Lazy::new(|| [[(0, "LeftHand")], [(0, "RightHand")]]);

static HAND_SKELETON_ROTATIONS: Lazy<[HashMap<usize, Quat>; 2]> = Lazy::new(|| {
    [
        HashMap::from([(
            0,
            Quat::from_xyzw(
                -6.213430570750633e-08,
                -1.7979416426202113e-07,
                0.7071067411992601,
                0.7071065897770331,
            ),
        )]),
        HashMap::from([(
            0,
            Quat::from_xyzw(
                0.3219087189747184,
                0.7288832784221684,
                0.3392694392278636,
                -0.4999999512887856,
            ),
        )]),
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
        &mut self,
        hand_type: HandType,
        mut skeleton: [Pose; 26],
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
        &mut self,
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
                        OscType::String(DEVICE_MOTIONS_VMC_MAP.get(id).unwrap().to_string()),
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
