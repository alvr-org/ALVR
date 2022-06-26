use alvr_common::glam::Vec3;
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

    // todo: move here more modifiers from C++
    pub fn map_head(&self, device_motion: DeviceMotion) -> DeviceMotion {
        DeviceMotion {
            position: if !self.settings.force_3dof {
                device_motion.position
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            },
            ..device_motion
        }
    }

    // todo: move here more modifiers from C++
    pub fn map_controller(&self, device_motion: DeviceMotion) -> Option<DeviceMotion> {
        if let Switch::Enabled(_) = &self.settings.controllers {
            Some(device_motion)
        } else {
            None
        }
    }
}
