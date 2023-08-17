use crate::{to_ffi_quat, FfiDeviceMotion, FfiHandSkeleton};
use alvr_common::{
    glam::{EulerRot, Quat, Vec2, Vec3},
    warn, DeviceMotion, Pose, A_CLICK_ID, B_CLICK_ID, HEAD_ID, LEFT_HAND_ID, LEFT_SQUEEZE_CLICK_ID,
    LEFT_SQUEEZE_VALUE_ID, LEFT_THUMBSTICK_CLICK_ID, LEFT_THUMBSTICK_TOUCH_ID,
    LEFT_THUMBSTICK_X_ID, LEFT_THUMBSTICK_Y_ID, LEFT_TRIGGER_CLICK_ID, LEFT_TRIGGER_VALUE_ID,
    MENU_CLICK_ID, RIGHT_HAND_ID, RIGHT_SQUEEZE_CLICK_ID, RIGHT_SQUEEZE_VALUE_ID,
    RIGHT_THUMBSTICK_CLICK_ID, RIGHT_THUMBSTICK_TOUCH_ID, RIGHT_THUMBSTICK_X_ID,
    RIGHT_THUMBSTICK_Y_ID, RIGHT_TRIGGER_CLICK_ID, RIGHT_TRIGGER_VALUE_ID, X_CLICK_ID, Y_CLICK_ID,
};
use alvr_session::{
    settings_schema::Switch, HeadsetConfig, PositionRecenteringMode, RotationRecenteringMode,
};
use std::{
    collections::HashMap,
    f32::consts::{FRAC_PI_2, PI},
};

const DEG_TO_RAD: f32 = PI / 180.0;

fn get_hand_skeleton_offsets(config: &HeadsetConfig) -> (Pose, Pose) {
    let left_offset;
    let right_offset;
    if let Switch::Enabled(controllers) = &config.controllers {
        let t = controllers.left_hand_tracking_position_offset;
        let r = controllers.left_hand_tracking_rotation_offset;

        left_offset = Pose {
            orientation: Quat::from_euler(
                EulerRot::XYZ,
                r[0] * DEG_TO_RAD,
                r[1] * DEG_TO_RAD,
                r[2] * DEG_TO_RAD,
            ),
            position: Vec3::new(t[0], t[1], t[2]),
        };
        right_offset = Pose {
            orientation: Quat::from_euler(
                EulerRot::XYZ,
                r[0] * DEG_TO_RAD,
                -r[1] * DEG_TO_RAD,
                -r[2] * DEG_TO_RAD,
            ),
            position: Vec3::new(-t[0], t[1], t[2]),
        };
    } else {
        left_offset = Pose::default();
        right_offset = Pose::default();
    }

    (left_offset, right_offset)
}

// todo: Move this struct to Settings and use it for every tracked device
#[derive(Default)]
struct MotionConfig {
    // Position offset applied after rotation offset
    pose_offset: Pose,
    linear_velocity_cutoff: f32,
    angular_velocity_cutoff: f32,
}

pub struct TrackingManager {
    last_head_pose: Pose,     // client's reference space
    recentering_origin: Pose, // client's reference space
}

impl TrackingManager {
    pub fn new() -> TrackingManager {
        TrackingManager {
            last_head_pose: Pose::default(),
            recentering_origin: Pose::default(),
        }
    }

    pub fn recenter(
        &mut self,
        position_recentering_mode: PositionRecenteringMode,
        rotation_recentering_mode: RotationRecenteringMode,
    ) {
        self.recentering_origin.position = match position_recentering_mode {
            PositionRecenteringMode::Disabled => Vec3::ZERO,
            PositionRecenteringMode::LocalFloor => {
                let mut pos = self.last_head_pose.position;
                pos.y = 0.0;

                pos
            }
            PositionRecenteringMode::Local { view_height } => {
                self.last_head_pose.position - Vec3::new(0.0, view_height, 0.0)
            }
        };

        self.recentering_origin.orientation = match rotation_recentering_mode {
            RotationRecenteringMode::Disabled => Quat::IDENTITY,
            RotationRecenteringMode::Yaw => {
                let mut rot = self.last_head_pose.orientation;
                // extract yaw rotation
                rot.x = 0.0;
                rot.z = 0.0;
                rot = rot.normalize();

                rot
            }
            RotationRecenteringMode::Tilted => self.last_head_pose.orientation,
        };
    }

    pub fn recenter_pose(&self, pose: Pose) -> Pose {
        let inverse_origin_orientation = self.recentering_origin.orientation.conjugate();

        Pose {
            orientation: inverse_origin_orientation * pose.orientation,
            position: inverse_origin_orientation
                * (pose.position - self.recentering_origin.position),
        }
    }

    // Performs all kinds of tracking transformations, driven by settings.
    pub fn transform_motions(
        &mut self,
        config: &HeadsetConfig,
        device_motions: &[(u64, DeviceMotion)],
        hand_skeletons_enabled: [bool; 2],
    ) -> Vec<(u64, DeviceMotion)> {
        let mut device_motion_configs = HashMap::new();
        device_motion_configs.insert(*HEAD_ID, MotionConfig::default());

        if let Switch::Enabled(controllers) = &config.controllers {
            let t = controllers.left_controller_position_offset;
            let r = controllers.left_controller_rotation_offset;

            device_motion_configs.insert(
                *LEFT_HAND_ID,
                MotionConfig {
                    pose_offset: Pose {
                        orientation: Quat::from_euler(
                            EulerRot::XYZ,
                            r[0] * DEG_TO_RAD,
                            r[1] * DEG_TO_RAD,
                            r[2] * DEG_TO_RAD,
                        ),
                        position: Vec3::new(t[0], t[1], t[2]),
                    },
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );

            device_motion_configs.insert(
                *RIGHT_HAND_ID,
                MotionConfig {
                    pose_offset: Pose {
                        orientation: Quat::from_euler(
                            EulerRot::XYZ,
                            r[0] * DEG_TO_RAD,
                            -r[1] * DEG_TO_RAD,
                            -r[2] * DEG_TO_RAD,
                        ),
                        position: Vec3::new(-t[0], t[1], t[2]),
                    },
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );
        }

        let (left_hand_skeleton_offset, right_hand_skeleton_offset) =
            get_hand_skeleton_offsets(config);

        let mut transformed_motions = vec![];
        for &(device_id, mut motion) in device_motions {
            if device_id == *HEAD_ID {
                self.last_head_pose = motion.pose;
            }

            if let Some(config) = device_motion_configs.get(&device_id) {
                // Recenter
                motion.pose = self.recenter_pose(motion.pose);

                let inverse_origin_orientation = self.recentering_origin.orientation.conjugate();
                motion.linear_velocity = inverse_origin_orientation * motion.linear_velocity;
                motion.angular_velocity = inverse_origin_orientation * motion.angular_velocity;

                // Apply custom transform
                let pose_offset = if device_id == *LEFT_HAND_ID && hand_skeletons_enabled[0] {
                    left_hand_skeleton_offset
                } else if device_id == *RIGHT_HAND_ID && hand_skeletons_enabled[1] {
                    right_hand_skeleton_offset
                } else {
                    config.pose_offset
                };
                motion.pose.orientation *= pose_offset.orientation;
                motion.pose.position += motion.pose.orientation * pose_offset.position;

                motion.linear_velocity += motion
                    .angular_velocity
                    .cross(motion.pose.orientation * pose_offset.position);
                motion.angular_velocity =
                    motion.pose.orientation.conjugate() * motion.angular_velocity;

                fn cutoff(v: Vec3, threshold: f32) -> Vec3 {
                    if v.length_squared() > threshold * threshold {
                        v
                    } else {
                        Vec3::ZERO
                    }
                }

                if (device_id == *LEFT_HAND_ID && hand_skeletons_enabled[0])
                    || (device_id == *RIGHT_HAND_ID && hand_skeletons_enabled[1])
                {
                    // On hand tracking, velocities seem to make hands overly jittery
                    motion.linear_velocity = Vec3::ZERO;
                    motion.angular_velocity = Vec3::ZERO;
                } else {
                    motion.linear_velocity =
                        cutoff(motion.linear_velocity, config.linear_velocity_cutoff);
                    motion.angular_velocity =
                        cutoff(motion.angular_velocity, config.angular_velocity_cutoff);
                }

                transformed_motions.push((device_id, motion));
            }
        }

        transformed_motions
    }
}

pub fn to_openvr_hand_skeleton(
    config: &HeadsetConfig,
    device_id: u64,
    hand_skeleton: [Pose; 26],
) -> [Pose; 26] {
    let (left_hand_skeleton_offset, right_hand_skeleton_offset) = get_hand_skeleton_offsets(config);

    // Convert from global to local joint pose. The orientation frame of reference is also
    // converted from OpenXR to SteamVR (hand-specific!)
    pub fn local_pose(id: u64, parent: Pose, current: Pose) -> Pose {
        let o = parent.orientation.conjugate() * current.orientation;
        let p = parent.orientation.conjugate() * (current.position - parent.position);

        // Convert to SteamVR frame of reference
        let (orientation, position) = if id == *LEFT_HAND_ID {
            (
                Quat::from_xyzw(-o.z, -o.y, -o.x, o.w),
                Vec3::new(-p.z, -p.y, -p.x),
            )
        } else {
            (
                Quat::from_xyzw(o.z, o.y, -o.x, o.w),
                Vec3::new(p.z, p.y, -p.x),
            )
        };

        Pose {
            orientation,
            position,
        }
    }

    let id = device_id;

    // global joints
    let gj = hand_skeleton;

    let fixed_g_wrist = Pose {
        orientation: gj[1].orientation
            * Quat::from_euler(EulerRot::YXZ, -FRAC_PI_2, FRAC_PI_2, 0.0),
        position: gj[1].position,
    };

    [
        // Palm. NB: this is ignored by SteamVR
        Pose::default(),
        // Wrist
        {
            let pose_offset = if device_id == *LEFT_HAND_ID {
                left_hand_skeleton_offset
            } else {
                right_hand_skeleton_offset
            };

            let sign = if id == *LEFT_HAND_ID { -1.0 } else { 1.0 };
            let orientation = pose_offset.orientation.conjugate()
                * gj[0].orientation.conjugate()
                * gj[1].orientation
                * Quat::from_euler(EulerRot::XZY, PI, sign * FRAC_PI_2, 0.0);

            let position = -pose_offset.position
                + pose_offset.orientation.conjugate()
                    * gj[0].orientation.conjugate()
                    * (gj[1].position - gj[0].position);

            Pose {
                orientation,
                position,
            }
        },
        // Thumb
        local_pose(id, fixed_g_wrist, gj[2]),
        local_pose(id, gj[2], gj[3]),
        local_pose(id, gj[3], gj[4]),
        local_pose(id, gj[4], gj[5]),
        // Index
        local_pose(id, fixed_g_wrist, gj[6]),
        local_pose(id, gj[6], gj[7]),
        local_pose(id, gj[7], gj[8]),
        local_pose(id, gj[8], gj[9]),
        local_pose(id, gj[9], gj[10]),
        // Middle
        local_pose(id, fixed_g_wrist, gj[11]),
        local_pose(id, gj[11], gj[12]),
        local_pose(id, gj[12], gj[13]),
        local_pose(id, gj[13], gj[14]),
        local_pose(id, gj[14], gj[15]),
        // Ring
        local_pose(id, fixed_g_wrist, gj[16]),
        local_pose(id, gj[16], gj[17]),
        local_pose(id, gj[17], gj[18]),
        local_pose(id, gj[18], gj[19]),
        local_pose(id, gj[19], gj[20]),
        // Little
        local_pose(id, fixed_g_wrist, gj[21]),
        local_pose(id, gj[21], gj[22]),
        local_pose(id, gj[22], gj[23]),
        local_pose(id, gj[23], gj[24]),
        local_pose(id, gj[24], gj[25]),
    ]
}

#[derive(Debug, Copy, Clone)]
pub struct HandGesture {
    pub active: bool,
    pub touching: bool,
    pub hover_val: f32,
    pub touch_bind: u64,
    pub hover_bind: u64,
}

pub fn hands_to_gestures(
    config: &HeadsetConfig,
    device_id: u64,
    hand_skeleton: [Pose; 26],
) -> [HandGesture; 8] {
    if let Switch::Enabled(controllers) = &config.controllers {
        if let Switch::Enabled(hand_tracking) = &controllers.hand_tracking {
            if let Switch::Enabled(use_gestures) = &hand_tracking.use_gestures {
                // global joints
                let gj = hand_skeleton;

                // if we model the tip of the finger as a spherical object, we should account for its radius
                // these are intentionally under the average by ~5mm since the touch and trigger distances are already configurable in settings
                let thumb_rad: f32 = 0.0075; // average thumb is ~20mm in diameter
                let index_rad: f32 = 0.0065; // average index finger is ~18mm in diameter
                let middle_rad: f32 = 0.0065; // average middle finger is ~18mm in diameter
                let ring_rad: f32 = 0.006; // average ring finger is ~17mm in diameter
                let little_rad: f32 = 0.005; // average pinky finger is ~15mm in diameter
                let palm_depth: f32 = 0.005; // average palm bones are ~10mm from the skin

                // we add the radius of the finger and thumb because we're measuring the distance between the surface of them, not their centers
                let pinch_min = use_gestures.pinch_touch_distance * 0.01;
                let pinch_max = use_gestures.pinch_trigger_distance * 0.01;
                let curl_min = use_gestures.curl_touch_distance * 0.01;
                let curl_max = use_gestures.curl_trigger_distance * 0.01;

                let palm: Pose = gj[0];
                let thumb_proximal: Pose = gj[3];
                let thumb_tip: Pose = gj[5];
                let index_metacarpal: Pose = gj[6];
                let index_proximal: Pose = gj[7];
                let index_intermediate: Pose = gj[8];
                let index_tip: Pose = gj[10];
                let middle_metacarpal: Pose = gj[11];
                let middle_proximal: Pose = gj[12];
                let middle_intermediate: Pose = gj[13];
                let middle_tip: Pose = gj[15];
                let ring_metacarpal: Pose = gj[16];
                let ring_proximal: Pose = gj[17];
                let ring_tip: Pose = gj[20];
                let little_metacarpal: Pose = gj[21];
                let little_proximal: Pose = gj[22];
                let little_tip: Pose = gj[25];

                let thumb_curl = (1.0
                    - (palm.position.distance(thumb_tip.position)
                        - curl_min
                        - palm_depth
                        - thumb_rad)
                        / (curl_max + palm_depth + thumb_rad))
                    .clamp(0.0, 1.0);

                let index_pinch = thumb_tip.position.distance(index_tip.position)
                    < pinch_min + thumb_rad + index_rad;
                let index_trigger = (1.0
                    - (thumb_tip.position.distance(index_tip.position)
                        - pinch_min
                        - thumb_rad
                        - index_rad)
                        / (pinch_max + thumb_rad + index_rad))
                    .clamp(0.0, 1.0);

                let index_curl = (1.0
                    - (index_metacarpal
                        .position
                        .lerp(index_proximal.position, 0.5)
                        .distance(index_tip.position)
                        - curl_min
                        - palm_depth
                        - index_rad)
                        / (curl_max + palm_depth + index_rad))
                    .clamp(0.0, 1.0);

                let middle_pinch = thumb_tip.position.distance(middle_tip.position)
                    < pinch_min + thumb_rad + middle_rad;
                let middle_trigger = (1.0
                    - (thumb_tip.position.distance(middle_tip.position)
                        - pinch_min
                        - thumb_rad
                        - middle_rad)
                        / (pinch_max + thumb_rad + middle_rad))
                    .clamp(0.0, 1.0);

                let middle_curl = (1.0
                    - (middle_metacarpal
                        .position
                        .lerp(middle_proximal.position, 0.5)
                        .distance(middle_tip.position)
                        - curl_min
                        - palm_depth
                        - middle_rad)
                        / (curl_max + palm_depth + middle_rad))
                    .clamp(0.0, 1.0);

                let ring_pinch = thumb_tip.position.distance(ring_tip.position)
                    < pinch_min + thumb_rad + ring_rad;
                let ring_trigger = (1.0
                    - (thumb_tip.position.distance(ring_tip.position)
                        - pinch_min
                        - thumb_rad
                        - ring_rad)
                        / (pinch_max + thumb_rad + ring_rad))
                    .clamp(0.0, 1.0);

                let ring_curl = (1.0
                    - (ring_metacarpal
                        .position
                        .lerp(ring_proximal.position, 0.5)
                        .distance(ring_tip.position)
                        - curl_min
                        - palm_depth
                        - ring_rad)
                        / (curl_max + palm_depth + ring_rad))
                    .clamp(0.0, 1.0);

                let little_pinch = thumb_tip.position.distance(little_tip.position)
                    < pinch_min + thumb_rad + little_rad;
                let little_trigger = (1.0
                    - (thumb_tip.position.distance(little_tip.position)
                        - pinch_min
                        - thumb_rad
                        - little_rad)
                        / (pinch_max + thumb_rad + little_rad))
                    .clamp(0.0, 1.0);

                let little_curl = (1.0
                    - (little_metacarpal
                        .position
                        .lerp(little_proximal.position, 0.5)
                        .distance(little_tip.position)
                        - curl_min
                        - palm_depth
                        - little_rad)
                        / (curl_max + palm_depth + little_rad))
                    .clamp(0.0, 1.0);

                let grip_curl = (middle_curl + ring_curl + little_curl) / 3.0;

                let joystick_range = 0.01;
                let joystick_center = index_intermediate.position.lerp(index_tip.position, 0.25);

                let joystick_up = (joystick_center
                    - middle_intermediate.position.lerp(middle_tip.position, 0.25))
                .normalize()
                    * joystick_range;

                let joystick_vertical_vec =
                    (joystick_center - thumb_proximal.position).normalize() * joystick_range;
                let joystick_horizontal_vec =
                    joystick_vertical_vec.cross(joystick_up).normalize() * joystick_range;

                let joystick_vertical = (thumb_tip.position - joystick_center
                    + joystick_vertical_vec / 2.0)
                    .dot(joystick_vertical_vec)
                    / joystick_vertical_vec.length();
                let joystick_horizontal = (thumb_tip.position - joystick_center)
                    .dot(joystick_horizontal_vec)
                    / joystick_horizontal_vec.length();

                let joystick_pos = Vec2 {
                    x: (joystick_horizontal / joystick_range).clamp(-1.0, 1.0),
                    y: (joystick_vertical / joystick_range).clamp(-1.0, 1.0),
                };
                let joystick_contact = index_curl >= 0.75
                    && grip_curl > 0.5
                    && joystick_center.distance(thumb_tip.position) <= joystick_range * 5.0
                    && (thumb_tip.position - joystick_center).dot(joystick_up)
                        / joystick_up.length()
                        <= joystick_range * 3.0;

                warn!("joystick contact: {}", joystick_contact);
                warn!(
                    "joystick position: {}, {}",
                    joystick_horizontal, joystick_vertical
                );
                warn!("joystick value: {}, {}", joystick_pos.x, joystick_pos.y);

                let joystick_deadzone = 0.25;

                return [
                    HandGesture {
                        active: true,
                        touching: index_pinch && !joystick_contact,
                        hover_val: if joystick_contact { 0.0 } else { index_trigger },
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_TRIGGER_CLICK_ID
                        } else {
                            *RIGHT_TRIGGER_CLICK_ID
                        },
                        hover_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_TRIGGER_VALUE_ID
                        } else {
                            *RIGHT_TRIGGER_VALUE_ID
                        },
                    },
                    HandGesture {
                        active: true,
                        touching: middle_pinch,
                        hover_val: middle_trigger,
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *Y_CLICK_ID
                        } else {
                            *B_CLICK_ID
                        },
                        hover_bind: 0,
                    },
                    HandGesture {
                        active: true,
                        touching: ring_pinch,
                        hover_val: ring_trigger,
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *X_CLICK_ID
                        } else {
                            *A_CLICK_ID
                        },
                        hover_bind: 0,
                    },
                    HandGesture {
                        active: true,
                        touching: little_pinch,
                        hover_val: little_trigger,
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *MENU_CLICK_ID
                        } else {
                            0
                        },
                        hover_bind: 0,
                    },
                    HandGesture {
                        active: true,
                        touching: grip_curl == 1.0,
                        hover_val: grip_curl,
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_SQUEEZE_CLICK_ID
                        } else {
                            *RIGHT_SQUEEZE_CLICK_ID
                        },
                        hover_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_SQUEEZE_VALUE_ID
                        } else {
                            *RIGHT_SQUEEZE_VALUE_ID
                        },
                    },
                    HandGesture {
                        active: true,
                        touching: thumb_curl == 1.0,
                        hover_val: thumb_curl,
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_THUMBSTICK_CLICK_ID
                        } else {
                            *RIGHT_THUMBSTICK_CLICK_ID
                        },
                        hover_bind: if device_id == *LEFT_HAND_ID { 0 } else { 0 },
                    },
                    HandGesture {
                        active: true,
                        touching: joystick_contact,
                        hover_val: if joystick_contact && joystick_pos.x >= joystick_deadzone {
                            joystick_pos.x
                        } else {
                            0.0
                        },
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_THUMBSTICK_TOUCH_ID
                        } else {
                            *RIGHT_THUMBSTICK_TOUCH_ID
                        },
                        hover_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_THUMBSTICK_X_ID
                        } else {
                            *RIGHT_THUMBSTICK_X_ID
                        },
                    },
                    HandGesture {
                        active: true,
                        touching: joystick_contact,
                        hover_val: if joystick_contact && joystick_pos.y >= joystick_deadzone {
                            joystick_pos.y
                        } else {
                            0.0
                        },
                        touch_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_THUMBSTICK_TOUCH_ID
                        } else {
                            *RIGHT_THUMBSTICK_TOUCH_ID
                        },
                        hover_bind: if device_id == *LEFT_HAND_ID {
                            *LEFT_THUMBSTICK_Y_ID
                        } else {
                            *RIGHT_THUMBSTICK_Y_ID
                        },
                    },
                ];
            }
        }
    }

    [
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
        HandGesture {
            active: false,
            touching: false,
            hover_val: 0.0,
            touch_bind: 0,
            hover_bind: 0,
        },
    ]
}

pub fn to_ffi_motion(device_id: u64, motion: DeviceMotion) -> FfiDeviceMotion {
    FfiDeviceMotion {
        deviceID: device_id,
        orientation: to_ffi_quat(motion.pose.orientation),
        position: motion.pose.position.to_array(),
        linearVelocity: motion.linear_velocity.to_array(),
        angularVelocity: motion.angular_velocity.to_array(),
    }
}

pub fn to_ffi_skeleton(skeleton: [Pose; 26]) -> FfiHandSkeleton {
    FfiHandSkeleton {
        jointRotations: skeleton
            .iter()
            .map(|j| to_ffi_quat(j.orientation))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        jointPositions: skeleton
            .iter()
            .map(|j| j.position.to_array())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    }
}

// Head and eyesmust be in the same (nt recentered) convention
pub fn to_local_eyes(
    raw_global_head: Pose,
    raw_global_eyes: [Option<Pose>; 2],
) -> [Option<Pose>; 2] {
    [
        raw_global_eyes[0].map(|e| raw_global_head.inverse() * e),
        raw_global_eyes[1].map(|e| raw_global_head.inverse() * e),
    ]
}
