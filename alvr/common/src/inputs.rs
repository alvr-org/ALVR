use crate::hash_string;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

// OpenXR interaction paths. They are used for the communication protocol and can also be used
// directly for OpenXR interop.
pub const HEAD_PATH: &str = "/user/head";
pub const LEFT_HAND_PATH: &str = "/user/hand/left";
pub const RIGHT_HAND_PATH: &str = "/user/hand/right";

pub const QUEST_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/oculus/touch_controller";
pub const VIVE_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/htc/vive_controller";
pub const INDEX_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/valve/index_controller";
pub const PICO_NEO3_CONTROLLER_PROFILE_PATH: &str =
    "/interaction_profiles/bytedance/pico_neo3_controller";
pub const PICO4_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/bytedance/pico4_controller";
pub const FOCUS3_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/htc/vive_focus3_controller";
pub const YVR_CONTROLLER_PROFILE_PATH: &str = "/interaction_profiles/yvr/touch_controller";

pub const HEAD_ENTER_CLICK_PATH: &str = "/user/head/input/enter/click";
pub const LEFT_SYSTEM_CLICK_PATH: &str = "/user/hand/left/input/system/click";
pub const LEFT_SYSTEM_TOUCH_PATH: &str = "/user/hand/left/input/system/touch";
pub const LEFT_MENU_CLICK_PATH: &str = "/user/hand/left/input/menu/click";
pub const LEFT_BACK_CLICK_PATH: &str = "/user/hand/left/input/back/click";
pub const LEFT_A_CLICK_PATH: &str = "/user/hand/left/input/a/click";
pub const LEFT_A_TOUCH_PATH: &str = "/user/hand/left/input/a/touch";
pub const LEFT_B_CLICK_PATH: &str = "/user/hand/left/input/b/click";
pub const LEFT_B_TOUCH_PATH: &str = "/user/hand/left/input/b/touch";
pub const LEFT_X_CLICK_PATH: &str = "/user/hand/left/input/x/click";
pub const LEFT_X_TOUCH_PATH: &str = "/user/hand/left/input/x/touch";
pub const LEFT_Y_CLICK_PATH: &str = "/user/hand/left/input/y/click";
pub const LEFT_Y_TOUCH_PATH: &str = "/user/hand/left/input/y/touch";
pub const LEFT_SQUEEZE_CLICK_PATH: &str = "/user/hand/left/input/squeeze/click";
pub const LEFT_SQUEEZE_TOUCH_PATH: &str = "/user/hand/left/input/squeeze/touch";
pub const LEFT_SQUEEZE_VALUE_PATH: &str = "/user/hand/left/input/squeeze/value";
pub const LEFT_SQUEEZE_FORCE_PATH: &str = "/user/hand/left/input/squeeze/force";
pub const LEFT_TRIGGER_CLICK_PATH: &str = "/user/hand/left/input/trigger/click";
pub const LEFT_TRIGGER_TOUCH_PATH: &str = "/user/hand/left/input/trigger/touch";
pub const LEFT_TRIGGER_VALUE_PATH: &str = "/user/hand/left/input/trigger/value";
pub const LEFT_THUMBSTICK_X_PATH: &str = "/user/hand/left/input/thumbstick/x";
pub const LEFT_THUMBSTICK_Y_PATH: &str = "/user/hand/left/input/thumbstick/y";
pub const LEFT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/left/input/thumbstick/click";
pub const LEFT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/left/input/thumbstick/touch";
pub const LEFT_TRACKPAD_X_PATH: &str = "/user/hand/left/input/trackpad/x";
pub const LEFT_TRACKPAD_Y_PATH: &str = "/user/hand/left/input/trackpad/y";
pub const LEFT_TRACKPAD_CLICK_PATH: &str = "/user/hand/left/input/trackpad/click";
pub const LEFT_TRACKPAD_FORCE_PATH: &str = "/user/hand/left/input/trackpad/force";
pub const LEFT_TRACKPAD_TOUCH_PATH: &str = "/user/hand/left/input/trackpad/touch";
pub const LEFT_THUMBREST_TOUCH_PATH: &str = "/user/hand/left/input/thumbrest/touch";

pub const RIGHT_SYSTEM_CLICK_PATH: &str = "/user/hand/right/input/system/click";
pub const RIGHT_SYSTEM_TOUCH_PATH: &str = "/user/hand/right/input/system/touch";
pub const RIGHT_MENU_CLICK_PATH: &str = "/user/hand/right/input/menu/click";
pub const RIGHT_BACK_CLICK_PATH: &str = "/user/hand/right/input/back/click";
pub const RIGHT_A_CLICK_PATH: &str = "/user/hand/right/input/a/click";
pub const RIGHT_A_TOUCH_PATH: &str = "/user/hand/right/input/a/touch";
pub const RIGHT_B_CLICK_PATH: &str = "/user/hand/right/input/b/click";
pub const RIGHT_B_TOUCH_PATH: &str = "/user/hand/right/input/b/touch";
pub const RIGHT_SQUEEZE_CLICK_PATH: &str = "/user/hand/right/input/squeeze/click";
pub const RIGHT_SQUEEZE_TOUCH_PATH: &str = "/user/hand/right/input/squeeze/touch";
pub const RIGHT_SQUEEZE_VALUE_PATH: &str = "/user/hand/right/input/squeeze/value";
pub const RIGHT_SQUEEZE_FORCE_PATH: &str = "/user/hand/right/input/squeeze/force";
pub const RIGHT_TRIGGER_CLICK_PATH: &str = "/user/hand/right/input/trigger/click";
pub const RIGHT_TRIGGER_VALUE_PATH: &str = "/user/hand/right/input/trigger/value";
pub const RIGHT_TRIGGER_TOUCH_PATH: &str = "/user/hand/right/input/trigger/touch";
pub const RIGHT_THUMBSTICK_X_PATH: &str = "/user/hand/right/input/thumbstick/x";
pub const RIGHT_THUMBSTICK_Y_PATH: &str = "/user/hand/right/input/thumbstick/y";
pub const RIGHT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/right/input/thumbstick/click";
pub const RIGHT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/right/input/thumbstick/touch";
pub const RIGHT_TRACKPAD_X_PATH: &str = "/user/hand/right/input/trackpad/x";
pub const RIGHT_TRACKPAD_Y_PATH: &str = "/user/hand/right/input/trackpad/y";
pub const RIGHT_TRACKPAD_CLICK_PATH: &str = "/user/hand/right/input/trackpad/click";
pub const RIGHT_TRACKPAD_FORCE_PATH: &str = "/user/hand/right/input/trackpad/force";
pub const RIGHT_TRACKPAD_TOUCH_PATH: &str = "/user/hand/right/input/trackpad/touch";
pub const RIGHT_THUMBREST_TOUCH_PATH: &str = "/user/hand/right/input/thumbrest/touch";

pub static HEAD_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_PATH));
pub static LEFT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_HAND_PATH));
pub static RIGHT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_HAND_PATH));

pub static QUEST_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(QUEST_CONTROLLER_PROFILE_PATH));
pub static VIVE_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(VIVE_CONTROLLER_PROFILE_PATH));
pub static INDEX_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(INDEX_CONTROLLER_PROFILE_PATH));
pub static PICO_NEO3_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(PICO_NEO3_CONTROLLER_PROFILE_PATH));
pub static PICO4_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(PICO4_CONTROLLER_PROFILE_PATH));
pub static FOCUS3_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(FOCUS3_CONTROLLER_PROFILE_PATH));
pub static YVR_CONTROLLER_PROFILE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(YVR_CONTROLLER_PROFILE_PATH));

pub static HEAD_ENTER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_ENTER_CLICK_PATH));
pub static LEFT_SYSTEM_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SYSTEM_CLICK_PATH));
pub static LEFT_SYSTEM_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SYSTEM_TOUCH_PATH));
pub static LEFT_MENU_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_MENU_CLICK_PATH));
pub static LEFT_BACK_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_BACK_CLICK_PATH));
pub static LEFT_A_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_A_CLICK_PATH));
pub static LEFT_A_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_A_TOUCH_PATH));
pub static LEFT_B_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_B_CLICK_PATH));
pub static LEFT_B_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_B_TOUCH_PATH));
pub static LEFT_X_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_X_CLICK_PATH));
pub static LEFT_X_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_X_TOUCH_PATH));
pub static LEFT_Y_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_Y_CLICK_PATH));
pub static LEFT_Y_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_Y_TOUCH_PATH));
pub static LEFT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_CLICK_PATH));
pub static LEFT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_VALUE_PATH));
pub static LEFT_SQUEEZE_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_TOUCH_PATH));
pub static LEFT_SQUEEZE_FORCE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_FORCE_PATH));
pub static LEFT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_CLICK_PATH));
pub static LEFT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_VALUE_PATH));
pub static LEFT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_TOUCH_PATH));
pub static LEFT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_X_PATH));
pub static LEFT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_Y_PATH));
pub static LEFT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_CLICK_PATH));
pub static LEFT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_TOUCH_PATH));
pub static LEFT_TRACKPAD_X_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRACKPAD_X_PATH));
pub static LEFT_TRACKPAD_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRACKPAD_Y_PATH));
pub static LEFT_TRACKPAD_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRACKPAD_CLICK_PATH));
pub static LEFT_TRACKPAD_FORCE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRACKPAD_FORCE_PATH));
pub static LEFT_TRACKPAD_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRACKPAD_TOUCH_PATH));
pub static LEFT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBREST_TOUCH_PATH));

pub static RIGHT_SYSTEM_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SYSTEM_CLICK_PATH));
pub static RIGHT_SYSTEM_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SYSTEM_TOUCH_PATH));
pub static RIGHT_MENU_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_MENU_CLICK_PATH));
pub static RIGHT_BACK_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_BACK_CLICK_PATH));
pub static RIGHT_A_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_A_CLICK_PATH));
pub static RIGHT_A_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_A_TOUCH_PATH));
pub static RIGHT_B_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_B_CLICK_PATH));
pub static RIGHT_B_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_B_TOUCH_PATH));
pub static RIGHT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_CLICK_PATH));
pub static RIGHT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_VALUE_PATH));
pub static RIGHT_SQUEEZE_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_TOUCH_PATH));
pub static RIGHT_SQUEEZE_FORCE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_FORCE_PATH));
pub static RIGHT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_CLICK_PATH));
pub static RIGHT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_VALUE_PATH));
pub static RIGHT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_TOUCH_PATH));
pub static RIGHT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_X_PATH));
pub static RIGHT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_Y_PATH));
pub static RIGHT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_CLICK_PATH));
pub static RIGHT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_TOUCH_PATH));
pub static RIGHT_TRACKPAD_X_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRACKPAD_X_PATH));
pub static RIGHT_TRACKPAD_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRACKPAD_Y_PATH));
pub static RIGHT_TRACKPAD_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_TRACKPAD_CLICK_PATH));
pub static RIGHT_TRACKPAD_FORCE_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_TRACKPAD_FORCE_PATH));
pub static RIGHT_TRACKPAD_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_TRACKPAD_TOUCH_PATH));
pub static RIGHT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBREST_TOUCH_PATH));

pub static DEVICE_ID_TO_PATH: Lazy<HashMap<u64, &str>> = Lazy::new(|| {
    [
        (*HEAD_ID, HEAD_PATH),
        (*LEFT_HAND_ID, LEFT_HAND_PATH),
        (*RIGHT_HAND_ID, RIGHT_HAND_PATH),
    ]
    .into_iter()
    .collect()
});

pub enum ButtonType {
    Binary,
    Scalar,
}

pub struct ButtonInfo {
    pub path: &'static str,
    pub button_type: ButtonType,
    pub device_id: u64,
}

pub static BUTTON_INFO: Lazy<HashMap<u64, ButtonInfo>> = Lazy::new(|| {
    [
        (
            *HEAD_ENTER_CLICK_ID,
            ButtonInfo {
                path: HEAD_ENTER_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *HEAD_ID,
            },
        ),
        (
            *LEFT_SYSTEM_CLICK_ID,
            ButtonInfo {
                path: LEFT_SYSTEM_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_SYSTEM_TOUCH_ID,
            ButtonInfo {
                path: LEFT_SYSTEM_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_MENU_CLICK_ID,
            ButtonInfo {
                path: LEFT_MENU_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_BACK_CLICK_ID,
            ButtonInfo {
                path: LEFT_BACK_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_A_CLICK_ID,
            ButtonInfo {
                path: LEFT_A_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_A_TOUCH_ID,
            ButtonInfo {
                path: LEFT_A_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_B_CLICK_ID,
            ButtonInfo {
                path: LEFT_B_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_B_TOUCH_ID,
            ButtonInfo {
                path: LEFT_B_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_X_CLICK_ID,
            ButtonInfo {
                path: LEFT_X_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_X_TOUCH_ID,
            ButtonInfo {
                path: LEFT_X_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_Y_CLICK_ID,
            ButtonInfo {
                path: LEFT_Y_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_Y_TOUCH_ID,
            ButtonInfo {
                path: LEFT_Y_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_SQUEEZE_CLICK_ID,
            ButtonInfo {
                path: LEFT_SQUEEZE_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_SQUEEZE_VALUE_ID,
            ButtonInfo {
                path: LEFT_SQUEEZE_VALUE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_SQUEEZE_TOUCH_ID,
            ButtonInfo {
                path: LEFT_SQUEEZE_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_SQUEEZE_FORCE_ID,
            ButtonInfo {
                path: LEFT_SQUEEZE_FORCE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRIGGER_CLICK_ID,
            ButtonInfo {
                path: LEFT_TRIGGER_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRIGGER_VALUE_ID,
            ButtonInfo {
                path: LEFT_TRIGGER_VALUE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRIGGER_TOUCH_ID,
            ButtonInfo {
                path: LEFT_TRIGGER_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_THUMBSTICK_X_ID,
            ButtonInfo {
                path: LEFT_THUMBSTICK_X_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_THUMBSTICK_Y_ID,
            ButtonInfo {
                path: LEFT_THUMBSTICK_Y_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_THUMBSTICK_CLICK_ID,
            ButtonInfo {
                path: LEFT_THUMBSTICK_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_THUMBSTICK_TOUCH_ID,
            ButtonInfo {
                path: LEFT_THUMBSTICK_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRACKPAD_X_ID,
            ButtonInfo {
                path: LEFT_TRACKPAD_X_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRACKPAD_Y_ID,
            ButtonInfo {
                path: LEFT_TRACKPAD_Y_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRACKPAD_CLICK_ID,
            ButtonInfo {
                path: LEFT_TRACKPAD_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRACKPAD_FORCE_ID,
            ButtonInfo {
                path: LEFT_TRACKPAD_FORCE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_TRACKPAD_TOUCH_ID,
            ButtonInfo {
                path: LEFT_TRACKPAD_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *LEFT_THUMBREST_TOUCH_ID,
            ButtonInfo {
                path: LEFT_THUMBREST_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *LEFT_HAND_ID,
            },
        ),
        (
            *RIGHT_SYSTEM_CLICK_ID,
            ButtonInfo {
                path: RIGHT_SYSTEM_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_SYSTEM_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_SYSTEM_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_MENU_CLICK_ID,
            ButtonInfo {
                path: RIGHT_MENU_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_BACK_CLICK_ID,
            ButtonInfo {
                path: RIGHT_BACK_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_A_CLICK_ID,
            ButtonInfo {
                path: RIGHT_A_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_A_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_A_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_B_CLICK_ID,
            ButtonInfo {
                path: RIGHT_B_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_B_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_B_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_SQUEEZE_CLICK_ID,
            ButtonInfo {
                path: RIGHT_SQUEEZE_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_SQUEEZE_VALUE_ID,
            ButtonInfo {
                path: RIGHT_SQUEEZE_VALUE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_SQUEEZE_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_SQUEEZE_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_SQUEEZE_FORCE_ID,
            ButtonInfo {
                path: RIGHT_SQUEEZE_FORCE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRIGGER_CLICK_ID,
            ButtonInfo {
                path: RIGHT_TRIGGER_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRIGGER_VALUE_ID,
            ButtonInfo {
                path: RIGHT_TRIGGER_VALUE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRIGGER_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_TRIGGER_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_THUMBSTICK_X_ID,
            ButtonInfo {
                path: RIGHT_THUMBSTICK_X_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_THUMBSTICK_Y_ID,
            ButtonInfo {
                path: RIGHT_THUMBSTICK_Y_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_THUMBSTICK_CLICK_ID,
            ButtonInfo {
                path: RIGHT_THUMBSTICK_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_THUMBSTICK_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_THUMBSTICK_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRACKPAD_X_ID,
            ButtonInfo {
                path: RIGHT_TRACKPAD_X_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRACKPAD_Y_ID,
            ButtonInfo {
                path: RIGHT_TRACKPAD_Y_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRACKPAD_CLICK_ID,
            ButtonInfo {
                path: RIGHT_TRACKPAD_CLICK_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRACKPAD_FORCE_ID,
            ButtonInfo {
                path: RIGHT_TRACKPAD_FORCE_PATH,
                button_type: ButtonType::Scalar,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_TRACKPAD_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_TRACKPAD_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
        (
            *RIGHT_THUMBREST_TOUCH_ID,
            ButtonInfo {
                path: RIGHT_THUMBREST_TOUCH_PATH,
                button_type: ButtonType::Binary,
                device_id: *RIGHT_HAND_ID,
            },
        ),
    ]
    .into_iter()
    .collect()
});

pub struct InteractionProfileInfo {
    pub path: &'static str,
    pub button_set: HashSet<u64>,
}

pub static CONTROLLER_PROFILE_INFO: Lazy<HashMap<u64, InteractionProfileInfo>> = Lazy::new(|| {
    [
        (
            *QUEST_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: QUEST_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_X_CLICK_ID,
                    *LEFT_X_TOUCH_ID,
                    *LEFT_Y_CLICK_ID,
                    *LEFT_Y_TOUCH_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_SQUEEZE_VALUE_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_THUMBREST_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_A_TOUCH_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_B_TOUCH_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_SQUEEZE_VALUE_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_THUMBREST_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *VIVE_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: VIVE_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_SYSTEM_CLICK_ID,
                    *LEFT_SQUEEZE_CLICK_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_TRIGGER_CLICK_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_TRACKPAD_X_ID,
                    *LEFT_TRACKPAD_Y_ID,
                    *LEFT_TRACKPAD_CLICK_ID,
                    *LEFT_TRACKPAD_TOUCH_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_SQUEEZE_CLICK_ID,
                    *RIGHT_MENU_CLICK_ID,
                    *RIGHT_TRIGGER_CLICK_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_TRACKPAD_X_ID,
                    *RIGHT_TRACKPAD_Y_ID,
                    *RIGHT_TRACKPAD_CLICK_ID,
                    *RIGHT_TRACKPAD_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *INDEX_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: INDEX_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_SYSTEM_CLICK_ID,
                    *LEFT_SYSTEM_TOUCH_ID,
                    *LEFT_A_CLICK_ID,
                    *LEFT_A_TOUCH_ID,
                    *LEFT_B_CLICK_ID,
                    *LEFT_B_TOUCH_ID,
                    *LEFT_SQUEEZE_VALUE_ID,
                    *LEFT_SQUEEZE_FORCE_ID,
                    *LEFT_TRIGGER_CLICK_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_TRACKPAD_X_ID,
                    *LEFT_TRACKPAD_Y_ID,
                    *LEFT_TRACKPAD_FORCE_ID,
                    *LEFT_TRACKPAD_TOUCH_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_SYSTEM_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_A_TOUCH_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_B_TOUCH_ID,
                    *RIGHT_SQUEEZE_VALUE_ID,
                    *RIGHT_SQUEEZE_FORCE_ID,
                    *RIGHT_TRIGGER_CLICK_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_TRACKPAD_X_ID,
                    *RIGHT_TRACKPAD_Y_ID,
                    *RIGHT_TRACKPAD_FORCE_ID,
                    *RIGHT_TRACKPAD_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *PICO_NEO3_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: PICO_NEO3_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_X_CLICK_ID,
                    *LEFT_X_TOUCH_ID,
                    *LEFT_Y_CLICK_ID,
                    *LEFT_Y_TOUCH_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_SYSTEM_CLICK_ID,
                    *LEFT_TRIGGER_CLICK_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_SQUEEZE_CLICK_ID,
                    *LEFT_SQUEEZE_VALUE_ID,
                    *LEFT_THUMBREST_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_A_TOUCH_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_B_TOUCH_ID,
                    *RIGHT_MENU_CLICK_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_TRIGGER_CLICK_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_SQUEEZE_CLICK_ID,
                    *RIGHT_SQUEEZE_VALUE_ID,
                    *RIGHT_THUMBREST_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *PICO4_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: PICO4_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_X_CLICK_ID,
                    *LEFT_X_TOUCH_ID,
                    *LEFT_Y_CLICK_ID,
                    *LEFT_Y_TOUCH_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_SYSTEM_CLICK_ID,
                    *LEFT_TRIGGER_CLICK_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_SQUEEZE_CLICK_ID,
                    *LEFT_SQUEEZE_VALUE_ID,
                    *LEFT_THUMBREST_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_A_TOUCH_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_B_TOUCH_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_TRIGGER_CLICK_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_SQUEEZE_CLICK_ID,
                    *RIGHT_SQUEEZE_VALUE_ID,
                    *RIGHT_THUMBREST_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *FOCUS3_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: FOCUS3_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_X_CLICK_ID,
                    *LEFT_Y_CLICK_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_SQUEEZE_CLICK_ID,
                    // *LEFT_SQUEEZE_TOUCH_ID, // not actually working
                    *LEFT_SQUEEZE_VALUE_ID,
                    *LEFT_TRIGGER_CLICK_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_THUMBREST_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_SQUEEZE_CLICK_ID,
                    // *RIGHT_SQUEEZE_TOUCH_ID, // not actually working
                    *RIGHT_SQUEEZE_VALUE_ID,
                    *RIGHT_TRIGGER_CLICK_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_THUMBREST_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
        (
            *YVR_CONTROLLER_PROFILE_ID,
            InteractionProfileInfo {
                path: YVR_CONTROLLER_PROFILE_PATH,
                button_set: [
                    *LEFT_X_CLICK_ID,
                    *LEFT_Y_CLICK_ID,
                    *LEFT_MENU_CLICK_ID,
                    *LEFT_SQUEEZE_CLICK_ID,
                    *LEFT_TRIGGER_TOUCH_ID,
                    *LEFT_TRIGGER_VALUE_ID,
                    *LEFT_THUMBSTICK_X_ID,
                    *LEFT_THUMBSTICK_Y_ID,
                    *LEFT_THUMBSTICK_CLICK_ID,
                    *LEFT_THUMBSTICK_TOUCH_ID,
                    *LEFT_THUMBREST_TOUCH_ID,
                    *RIGHT_A_CLICK_ID,
                    *RIGHT_B_CLICK_ID,
                    *RIGHT_SYSTEM_CLICK_ID,
                    *RIGHT_SQUEEZE_CLICK_ID,
                    *RIGHT_TRIGGER_TOUCH_ID,
                    *RIGHT_TRIGGER_VALUE_ID,
                    *RIGHT_THUMBSTICK_X_ID,
                    *RIGHT_THUMBSTICK_Y_ID,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                    *RIGHT_THUMBREST_TOUCH_ID,
                ]
                .into_iter()
                .collect(),
            },
        ),
    ]
    .into_iter()
    .collect()
});
