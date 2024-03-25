use alvr_common::{
    anyhow::Result, once_cell::sync::Lazy, DeviceMotion, BODY_CHEST_ID, BODY_HIPS_ID,
    BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID,
    BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, HEAD_ID,
};
use alvr_session::BodyTrackingSinkConfig;
use rosc::{OscMessage, OscPacket, OscType};
use std::{collections::HashMap, net::UdpSocket};

use crate::tracking::TrackingManager;

static BODY_TRACKER_OSC_PATH_MAP: Lazy<HashMap<u64, &'static str>> = Lazy::new(|| {
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
            BodyTrackingSinkConfig::FakeViveTracker {} => Ok(Self {
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

    pub fn send_tracking(
        &mut self,
        device_motions: &[(u64, DeviceMotion)],
        tracking_manager: &TrackingManager,
    ) {
        match self.config {
            BodyTrackingSinkConfig::VrchatBodyOsc { .. } => {
                for (id, motion) in device_motions.iter() {
                    if BODY_TRACKER_OSC_PATH_MAP.contains_key(id) {
                        // Only do position because rotation isn't quite right
                        let pose = tracking_manager.recenter_pose(motion.pose);
                        self.send_osc_message(
                            format!(
                                "{}{}",
                                BODY_TRACKER_OSC_PATH_MAP.get(id).unwrap(),
                                "position"
                            )
                            .as_str(),
                            vec![
                                OscType::Float(pose.position.x),
                                OscType::Float(pose.position.y),
                                OscType::Float(-pose.position.z),
                            ],
                        );
                    }
                }
            }
            BodyTrackingSinkConfig::FakeViveTracker => {}
        }
    }
}
