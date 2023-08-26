use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    Pose, A_CLICK_ID, B_CLICK_ID, LEFT_HAND_ID, LEFT_SQUEEZE_CLICK_ID, LEFT_SQUEEZE_VALUE_ID,
    LEFT_THUMBSTICK_CLICK_ID, LEFT_THUMBSTICK_TOUCH_ID, LEFT_THUMBSTICK_X_ID, LEFT_THUMBSTICK_Y_ID,
    LEFT_TRIGGER_CLICK_ID, LEFT_TRIGGER_VALUE_ID, MENU_CLICK_ID, RIGHT_SQUEEZE_CLICK_ID,
    RIGHT_SQUEEZE_VALUE_ID, RIGHT_THUMBSTICK_CLICK_ID, RIGHT_THUMBSTICK_TOUCH_ID,
    RIGHT_THUMBSTICK_X_ID, RIGHT_THUMBSTICK_Y_ID, RIGHT_TRIGGER_CLICK_ID, RIGHT_TRIGGER_VALUE_ID,
    X_CLICK_ID, Y_CLICK_ID,
};

use alvr_session::HandGestureConfig;

fn lerp_pose(a: Pose, b: Pose, fac: f32) -> Pose {
    Pose {
        orientation: Quat::from_xyzw(
            a.orientation.x * (1.0 - fac) + (b.position.x * fac),
            a.orientation.y * (1.0 - fac) + (b.orientation.y * fac),
            a.orientation.z * (1.0 - fac) + (b.orientation.z * fac),
            a.orientation.w * (1.0 - fac) + (b.orientation.w * fac),
        ),
        position: Vec3 {
            x: a.position.x * (1.0 - fac) + (b.position.x * fac),
            y: a.position.y * (1.0 - fac) + (b.position.y * fac),
            z: a.position.z * (1.0 - fac) + (b.position.z * fac),
        },
    }
}

#[derive(Debug, Clone)]
pub struct HandGesture {
    pub id: HandGestureId,
    pub active: bool,
    pub hover: f32,
}

pub struct GestureAction {
    last_activated: u128,
    last_deactivated: u128,
    entering: bool,
    entering_since: u128,
    exiting: bool,
    exiting_since: u128,
    active: bool,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[allow(dead_code)]
pub enum HandGestureId {
    // Pinches
    ThumbIndexPinch,
    ThumbMiddlePinch,
    ThumbRingPinch,
    ThumbLittlePinch,
    // Curls
    ThumbCurl,
    IndexCurl,
    MiddleCurl,
    RingCurl,
    LittleCurl,
    GripCurl,
    // Complex
    JoystickX,
    JoystickY,
}

pub struct HandGestureManager {
    gesture_data_left: HashMap<HandGestureId, GestureAction>,
    gesture_data_right: HashMap<HandGestureId, GestureAction>,
}

impl HandGestureManager {
    pub fn new() -> Self {
        Self {
            gesture_data_left: HashMap::new(),
            gesture_data_right: HashMap::new(),
        }
    }

    pub fn get_active_gestures(
        &mut self,
        hand_skeleton: [Pose; 26],
        config: HandGestureConfig,
        device_id: u64,
    ) -> Vec<HandGesture> {
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
        let pinch_min = config.pinch_touch_distance * 0.01;
        let pinch_max = config.pinch_trigger_distance * 0.01;
        let curl_min = config.curl_touch_distance * 0.01;
        let curl_max = config.curl_trigger_distance * 0.01;

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

        let mut gestures: Vec<HandGesture> = Vec::new();

        // Thumb & index pinch
        gestures.push(HandGesture {
            id: HandGestureId::ThumbIndexPinch,
            active: self.is_gesture_active(
                HandGestureId::ThumbIndexPinch,
                thumb_tip,
                thumb_rad,
                index_tip,
                index_rad,
                pinch_max,
                config.repeat_delay,
                config.start_delay,
                config.stop_delay,
                device_id,
            ),
            hover: self.get_gesture_hover(
                thumb_tip, thumb_rad, index_tip, index_rad, pinch_min, pinch_max,
            ),
        });

        // Thumb & middle pinch
        gestures.push(HandGesture {
            id: HandGestureId::ThumbMiddlePinch,
            active: self.is_gesture_active(
                HandGestureId::ThumbMiddlePinch,
                thumb_tip,
                thumb_rad,
                middle_tip,
                middle_rad,
                pinch_max,
                config.repeat_delay,
                config.start_delay,
                config.stop_delay,
                device_id,
            ),
            hover: self.get_gesture_hover(
                thumb_tip, thumb_rad, middle_tip, middle_rad, pinch_min, pinch_max,
            ),
        });

        // Thumb & ring pinch
        gestures.push(HandGesture {
            id: HandGestureId::ThumbRingPinch,
            active: self.is_gesture_active(
                HandGestureId::ThumbRingPinch,
                thumb_tip,
                thumb_rad,
                ring_tip,
                ring_rad,
                pinch_max,
                config.repeat_delay,
                config.start_delay,
                config.stop_delay,
                device_id,
            ),
            hover: self.get_gesture_hover(
                thumb_tip, thumb_rad, ring_tip, ring_rad, pinch_min, pinch_max,
            ),
        });

        // Thumb & little pinch
        gestures.push(HandGesture {
            id: HandGestureId::ThumbLittlePinch,
            active: self.is_gesture_active(
                HandGestureId::ThumbLittlePinch,
                thumb_tip,
                thumb_rad,
                little_tip,
                little_rad,
                pinch_max,
                config.repeat_delay,
                config.start_delay,
                config.stop_delay,
                device_id,
            ),
            hover: self.get_gesture_hover(
                thumb_tip, thumb_rad, little_tip, little_rad, pinch_min, pinch_max,
            ),
        });

        // Finger curls
        let thumb_curl =
            self.get_gesture_hover(palm, palm_depth, thumb_tip, thumb_rad, curl_min, curl_max);
        let index_curl = self.get_gesture_hover(
            lerp_pose(index_metacarpal, index_proximal, 0.5),
            palm_depth,
            index_tip,
            index_rad,
            curl_min,
            curl_max,
        );
        let middle_curl = self.get_gesture_hover(
            lerp_pose(middle_metacarpal, middle_proximal, 0.5),
            palm_depth,
            middle_tip,
            middle_rad,
            curl_min,
            curl_max,
        );
        let ring_curl = self.get_gesture_hover(
            lerp_pose(ring_metacarpal, ring_proximal, 0.5),
            palm_depth,
            ring_tip,
            ring_rad,
            curl_min,
            curl_max,
        );
        let little_curl = self.get_gesture_hover(
            lerp_pose(little_metacarpal, little_proximal, 0.5),
            palm_depth,
            little_tip,
            little_rad,
            curl_min,
            curl_max,
        );

        // Grip (NEEDS REFINING)
        let grip_curl = (middle_curl + ring_curl + little_curl) / 3.0;
        let grip_active = grip_curl == 1.0;

        gestures.push(HandGesture {
            id: HandGestureId::GripCurl,
            active: grip_active,
            hover: grip_curl,
        });

        // Joystick (NEEDS REFINING)
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
            / joystick_vertical_vec.length()
            + config.joystick_offset_vertical * 0.01;
        let joystick_horizontal = (thumb_tip.position - joystick_center)
            .dot(joystick_horizontal_vec)
            / joystick_horizontal_vec.length()
            + config.joystick_offset_horizontal * 0.01;

        let joystick_pos = Vec2 {
            x: (joystick_horizontal / joystick_range).clamp(-1.0, 1.0),
            y: (joystick_vertical / joystick_range).clamp(-1.0, 1.0),
        };
        let joystick_contact = index_curl >= 0.75
            && grip_curl > 0.5
            && joystick_center.distance(thumb_tip.position) <= joystick_range * 5.0
            && (thumb_tip.position - joystick_center).dot(joystick_up) / joystick_up.length()
                <= joystick_range * 3.0;

        let joystick_deadzone: f32 = config.joystick_deadzone * 0.01;

        gestures.push(HandGesture {
            id: HandGestureId::ThumbCurl,
            active: thumb_curl == 1.0,
            hover: thumb_curl,
        });
        gestures.push(HandGesture {
            id: HandGestureId::JoystickX,
            active: joystick_contact,
            hover: if joystick_contact && joystick_pos.y >= joystick_deadzone {
                joystick_pos.x
            } else {
                0.0
            },
        });
        gestures.push(HandGesture {
            id: HandGestureId::JoystickY,
            active: joystick_contact,
            hover: if joystick_contact && joystick_pos.y >= joystick_deadzone {
                joystick_pos.y
            } else {
                0.0
            },
        });

        gestures
    }

    fn is_gesture_active(
        &mut self,
        gesture_id: HandGestureId,
        first_anchor: Pose,
        first_radius: f32,
        second_anchor: Pose,
        second_radius: f32,
        activation_dist: f32,
        repeat_delay: u32,
        in_delay: u32,
        out_delay: u32,
        device_id: u64,
    ) -> bool {
        let in_range = first_anchor.position.distance(second_anchor.position)
            < (activation_dist + first_radius + second_radius);

        let gesture_data = if device_id == *LEFT_HAND_ID {
            &mut self.gesture_data_left
        } else {
            &mut self.gesture_data_right
        };

        if !gesture_data.contains_key(&gesture_id) {
            gesture_data.insert(
                gesture_id,
                GestureAction {
                    last_activated: 0,
                    last_deactivated: 0,
                    entering: false,
                    entering_since: 0,
                    exiting: false,
                    exiting_since: 0,
                    active: false,
                },
            );
        }
        let g: &mut GestureAction = gesture_data.get_mut(&gesture_id).unwrap();

        // Disable entering/exiting state if we leave/enter range
        if in_range {
            g.exiting = false;
        } else {
            g.entering = false;
        }

        // Default to maintaining state
        let mut new_active_state = g.active;

        // Get current time, for comparison
        let time_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_millis(0))
            .as_millis();

        // Transitioning from inactive to active
        if in_range && !g.active {
            // Don't transition state unless the duration of repeat_delay has passed since last deactivation
            if g.last_deactivated < time_millis - u128::from(repeat_delay) {
                if g.entering {
                    // Don't transition state unless gesture has been in range for the duration of in_delay
                    if g.entering_since < time_millis - u128::from(in_delay) {
                        g.last_activated = time_millis;
                        g.entering = false;
                        new_active_state = true;
                    }
                } else {
                    // Begin tracking entering state
                    g.entering = true;
                    g.entering_since = time_millis;
                }
            }
        }

        // Transitioning from inactive to active
        if !in_range && g.active {
            if g.exiting {
                // Don't transition state unless gesture has been out of range for the duration of out_delay
                if g.exiting_since < time_millis - u128::from(out_delay) {
                    g.last_deactivated = time_millis;
                    g.exiting = false;
                    new_active_state = false;
                }
            } else {
                // Begin tracking exiting state
                g.exiting = true;
                g.exiting_since = time_millis;
            }
        }

        g.active = new_active_state;

        g.active
    }

    fn get_gesture_hover(
        &self,
        first_anchor: Pose,
        first_radius: f32,
        second_anchor: Pose,
        second_radius: f32,
        min_dist: f32,
        max_dist: f32,
    ) -> f32 {
        (1.0 - (first_anchor.position.distance(second_anchor.position)
            - min_dist
            - first_radius
            - second_radius)
            / (max_dist + first_radius + second_radius))
            .clamp(0.0, 1.0)
    }
}

fn get_active_bind_for_gesture(device_id: u64, gesture_id: HandGestureId) -> Option<u64> {
    if device_id == *LEFT_HAND_ID {
        match gesture_id {
            HandGestureId::ThumbIndexPinch => Some(*LEFT_TRIGGER_CLICK_ID),
            HandGestureId::ThumbMiddlePinch => Some(*Y_CLICK_ID),
            HandGestureId::ThumbRingPinch => Some(*X_CLICK_ID),
            HandGestureId::ThumbLittlePinch => Some(*MENU_CLICK_ID),
            HandGestureId::GripCurl => Some(*LEFT_SQUEEZE_CLICK_ID),
            HandGestureId::ThumbCurl => Some(*LEFT_THUMBSTICK_CLICK_ID),
            HandGestureId::JoystickX => Some(*LEFT_THUMBSTICK_TOUCH_ID),
            HandGestureId::JoystickY => Some(*LEFT_THUMBSTICK_TOUCH_ID),
            _ => None,
        }
    } else {
        match gesture_id {
            HandGestureId::ThumbIndexPinch => Some(*RIGHT_TRIGGER_CLICK_ID),
            HandGestureId::ThumbMiddlePinch => Some(*B_CLICK_ID),
            HandGestureId::ThumbRingPinch => Some(*A_CLICK_ID),
            HandGestureId::GripCurl => Some(*RIGHT_SQUEEZE_CLICK_ID),
            HandGestureId::ThumbCurl => Some(*RIGHT_THUMBSTICK_CLICK_ID),
            HandGestureId::JoystickX => Some(*RIGHT_THUMBSTICK_TOUCH_ID),
            HandGestureId::JoystickY => Some(*RIGHT_THUMBSTICK_TOUCH_ID),
            _ => None,
        }
    }
}

fn get_hover_bind_for_gesture(device_id: u64, gesture_id: HandGestureId) -> Option<u64> {
    if device_id == *LEFT_HAND_ID {
        match gesture_id {
            HandGestureId::ThumbIndexPinch => Some(*LEFT_TRIGGER_VALUE_ID),
            HandGestureId::GripCurl => Some(*LEFT_SQUEEZE_VALUE_ID),
            HandGestureId::JoystickX => Some(*LEFT_THUMBSTICK_X_ID),
            HandGestureId::JoystickY => Some(*LEFT_THUMBSTICK_Y_ID),
            _ => None,
        }
    } else {
        match gesture_id {
            HandGestureId::ThumbIndexPinch => Some(*RIGHT_TRIGGER_VALUE_ID),
            HandGestureId::GripCurl => Some(*RIGHT_SQUEEZE_VALUE_ID),
            HandGestureId::JoystickX => Some(*RIGHT_THUMBSTICK_X_ID),
            HandGestureId::JoystickY => Some(*RIGHT_THUMBSTICK_Y_ID),
            _ => None,
        }
    }
}

pub fn trigger_hand_gesture_actions(device_id: u64, gestures: Vec<HandGesture>) {
    for gesture in gestures.iter() {
        // Active bind
        let active_bind = get_active_bind_for_gesture(device_id, gesture.id);
        if active_bind.is_some() {
            unsafe {
                crate::SetButton(
                    active_bind.unwrap(),
                    crate::FfiButtonValue {
                        type_: crate::FfiButtonType_BUTTON_TYPE_BINARY,
                        __bindgen_anon_1: crate::FfiButtonValue__bindgen_ty_1 {
                            binary: gesture.active.into(),
                        },
                    },
                );
            }
        }

        // Hover bind
        let hover_bind = get_hover_bind_for_gesture(device_id, gesture.id);
        if hover_bind.is_some() {
            unsafe {
                crate::SetButton(
                    hover_bind.unwrap(),
                    crate::FfiButtonValue {
                        type_: crate::FfiButtonType_BUTTON_TYPE_BINARY,
                        __bindgen_anon_1: crate::FfiButtonValue__bindgen_ty_1 {
                            scalar: gesture.hover.into(),
                        },
                    },
                );
            }
        }
    }
}
