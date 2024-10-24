use alvr_common::{
    anyhow::Result, once_cell::sync::Lazy, DeviceMotion, BODY_CHEST_ID, BODY_HIPS_ID,
    BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID,
    BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket};

use alvr_session::VMCSinkConfig;

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
        (*BODY_RIGHT_KNEE_ID, "RightLowerLeg"),
        (*BODY_LEFT_FOOT_ID, "LeftFoot"),
        (*BODY_RIGHT_FOOT_ID, "RightFoot"),
        (*HEAD_ID, "Head"),
    ])
});

pub struct VMCSink {
    socket: Option<UdpSocket>,
}

impl VMCSink {
    pub fn new(config: VMCSinkConfig) -> Result<Self> {
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

    pub fn send_tracking(
        &mut self,
        device_motions: &[(u64, DeviceMotion)],
    ) {
        for (id, motion) in device_motions {
            if DEVICE_MOTIONS_VMC_MAP.contains_key(id) {
                self.send_osc_message(
                    "/VMC/Ext/Bone/Pos",
                    vec![
                        OscType::String(DEVICE_MOTIONS_VMC_MAP.get(id).unwrap().to_string()),
                        OscType::Float(motion.pose.position.x),
                        OscType::Float(motion.pose.position.y),
                        OscType::Float(motion.pose.position.z),
                        OscType::Float(motion.pose.orientation.x),
                        OscType::Float(motion.pose.orientation.y),
                        OscType::Float(motion.pose.orientation.z),
                        OscType::Float(motion.pose.orientation.w),
                    ],
                );
            }
        }
    }
}
