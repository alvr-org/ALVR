use alvr_common::{
    anyhow::Result, once_cell::sync::Lazy, DeviceMotion
};
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket};

use alvr_session::VMCSinkConfig;

// Transform DeviceMotion into Unity HumanBodyBones
// https://docs.unity3d.com/ScriptReference/HumanBodyBones.html
static DEVICE_MOTIONS_VMC_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("/user/hand/left", "LeftHand"),
        ("/user/hand/right", "RightHand"),
        ("/user/body/chest", "Chest"),
        ("/user/body/waist", "Hips"), //???
        ("/user/body/left_elbow", "LeftLowerArm"),
        ("/user/body/right_elbow", "RightLowerArm"),
        ("/user/body/left_knee", "LeftLowerLeg"),
        ("/user/body/right_knee", "RightLowerLeg"),
        ("/user/body/left_foot", "LeftFoot"),
        ("/user/body/right_foot", "RightFoot"),
        ("/user/head", "Head"),
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
        device_motions: Vec<(String, DeviceMotion)>,
    ) {
        for (id, motion) in device_motions {
            let sid = id.as_str();
            if DEVICE_MOTIONS_VMC_MAP.contains_key(sid) {
                self.send_osc_message(
                    "/VMC/Ext/Bone/Pos",
                    vec![
                        OscType::String(DEVICE_MOTIONS_VMC_MAP.get(sid).unwrap().to_string()),
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
