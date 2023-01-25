use alvr_common::{glam::Vec3, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID};
use alvr_session::HeadsetDesc;
use alvr_sockets::DeviceMotion;
use settings_schema::Switch;

pub struct TrackingManager {
    settings: HeadsetDesc,
}

impl TrackingManager {
    pub fn new(settings: HeadsetDesc) -> TrackingManager {
        TrackingManager { settings }
    }

    // todo: extend this method for space recalibration.
    pub fn filter_map_motion(
        &self,
        device_id: u64,
        mut motion: DeviceMotion,
    ) -> Option<DeviceMotion> {
        if device_id == *HEAD_ID {
            if self.settings.force_3dof {
                motion.pose.position = Vec3::ZERO;
            }

            Some(motion)
        } else if device_id == *LEFT_HAND_ID || device_id == *RIGHT_HAND_ID {
            matches!(self.settings.controllers, Switch::Enabled(_)).then(|| motion)
        } else {
            Some(motion)
        }
    }
}
