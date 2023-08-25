use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH}, hash::Hash,
};

use alvr_common::{anyhow::Result, Pose, glam::Vec2};
use alvr_session::HandGestureConfig;

#[derive(Debug, Copy, Clone)]
pub struct HandGesture {
    pub touching: bool,
    pub hover_val: f32,
    pub touch_bind: u64,
    pub hover_bind: u64,
}

pub struct GestureAction {
    last_activated: u128,
    last_deactivated: u128,
    active: bool,
}

#[derive(Eq, Hash, PartialEq)]
enum HandGestureId {}

pub struct HandGestureManager {
    config: HandGestureConfig,
    gesture_data: HashMap<HandGestureId, GestureAction>,
}

impl HandGestureManager {
    fn new(config: HandGestureConfig) -> Result<Self> {
        Ok(Self {
            config,
            gesture_data: HashMap::new()
        })
    }

    pub fn get_active_gestures(&self, hand_skeleton: [Pose; 26]) -> Vec<HandGesture> {
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
        let pinch_min = self.config.pinch_touch_distance * 0.01;
        let pinch_max = self.config.pinch_trigger_distance * 0.01;
        let curl_min = self.config.curl_touch_distance * 0.01;
        let curl_max = self.config.curl_trigger_distance * 0.01;

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
            - (palm.position.distance(thumb_tip.position) - curl_min - palm_depth - thumb_rad)
                / (curl_max + palm_depth + thumb_rad))
            .clamp(0.0, 1.0);

        let index_pinch =
            thumb_tip.position.distance(index_tip.position) < pinch_min + thumb_rad + index_rad;
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

        let middle_pinch =
            thumb_tip.position.distance(middle_tip.position) < pinch_min + thumb_rad + middle_rad;
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

        let ring_pinch =
            thumb_tip.position.distance(ring_tip.position) < pinch_min + thumb_rad + ring_rad;
        let ring_trigger = (1.0
            - (thumb_tip.position.distance(ring_tip.position) - pinch_min - thumb_rad - ring_rad)
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

        let little_pinch =
            thumb_tip.position.distance(little_tip.position) < pinch_min + thumb_rad + little_rad;
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
            / joystick_vertical_vec.length()
            + self.config.joystick_offset_vertical * 0.01;
        let joystick_horizontal = (thumb_tip.position - joystick_center)
            .dot(joystick_horizontal_vec)
            / joystick_horizontal_vec.length()
            + self.config.joystick_offset_horizontal * 0.01;

        let joystick_pos = Vec2 {
            x: (joystick_horizontal / joystick_range).clamp(-1.0, 1.0),
            y: (joystick_vertical / joystick_range).clamp(-1.0, 1.0),
        };
        let joystick_contact = index_curl >= 0.75
            && grip_curl > 0.5
            && joystick_center.distance(thumb_tip.position) <= joystick_range * 5.0
            && (thumb_tip.position - joystick_center).dot(joystick_up) / joystick_up.length()
                <= joystick_range * 3.0;

        let joystick_deadzone: f32 = self.config.joystick_deadzone * 0.01;

        return vec![];
    }

    fn is_gesture_active(
        &self,
        gesture_id: HandGestureId,
        first_anchor: Pose,
        first_rad: f32,
        second_anchor: Pose,
        second_rad: f32,
        halo: f32,
        in_delay: u128,
        out_delay: u128,
    ) -> bool {
        let is_active = first_anchor.position.distance(second_anchor.position)
            < (halo + first_rad + second_rad);

        if !self.gesture_data.contains_key(&gesture_id) {
            self.gesture_data.insert(gesture_id, GestureAction {
                last_activated: 0,
                last_deactivated: 0,
                active: false,
            });
        }
        let g = self.gesture_data.get(&gesture_id).unwrap();

        // If was already active, maintain state
        if is_active && g.active {
            return true;
        }
        // If was already inactive, maintain state
        if !is_active && !g.active {
            return false;
        }

        // Get current time, for comparison
        let time_millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

        // If is becoming active, do not transition state unless the in_delay has passed since last deactivation
        if is_active && !g.active {
            if g.last_deactivated < time_millis - in_delay {
                g.active = true;
                return true;
            } else {
                return false;
            }
        }

        // If is becoming inactive, do not transition state unless the out_delay has passed since last activation
        if !is_active && g.active {
            if g.last_activated < time_millis - out_delay {
                g.active = false;
                return false;
            } else {
                return true;
            }
        }
    }
}
