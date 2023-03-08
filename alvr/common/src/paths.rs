use crate::hash_string;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const HEAD_PATH: &str = "/user/head";
pub const LEFT_HAND_PATH: &str = "/user/hand/left";
pub const RIGHT_HAND_PATH: &str = "/user/hand/right";

pub static HEAD_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_PATH));
pub static LEFT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_HAND_PATH));
pub static RIGHT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_HAND_PATH));

pub static DEVICE_ID_TO_PATH: Lazy<HashMap<u64, &str>> = Lazy::new(|| {
    [
        (*HEAD_ID, HEAD_PATH),
        (*LEFT_HAND_ID, LEFT_HAND_PATH),
        (*RIGHT_HAND_ID, RIGHT_HAND_PATH),
    ]
    .into_iter()
    .collect()
});

pub const HEAD_ENTER_CLICK_PATH: &str = "/user/head/input/enter/click";
pub const BACK_CLICK_PATH: &str = "/user/hand/left/input/back/click";
pub const MENU_CLICK_PATH: &str = "/user/hand/left/input/menu/click";
pub const A_CLICK_PATH: &str = "/user/hand/right/input/a/click";
pub const A_TOUCH_PATH: &str = "/user/hand/right/input/a/touch";
pub const B_CLICK_PATH: &str = "/user/hand/right/input/b/click";
pub const B_TOUCH_PATH: &str = "/user/hand/right/input/b/touch";
pub const X_CLICK_PATH: &str = "/user/hand/left/input/x/click";
pub const X_TOUCH_PATH: &str = "/user/hand/left/input/x/touch";
pub const Y_CLICK_PATH: &str = "/user/hand/left/input/y/click";
pub const Y_TOUCH_PATH: &str = "/user/hand/left/input/y/touch";
pub const LEFT_SQUEEZE_CLICK_PATH: &str = "/user/hand/left/input/squeeze/click";
pub const LEFT_SQUEEZE_VALUE_PATH: &str = "/user/hand/left/input/squeeze/value";
pub const LEFT_TRIGGER_CLICK_PATH: &str = "/user/hand/left/input/trigger/click";
pub const LEFT_TRIGGER_VALUE_PATH: &str = "/user/hand/left/input/trigger/value";
pub const LEFT_TRIGGER_TOUCH_PATH: &str = "/user/hand/left/input/trigger/touch";
pub const LEFT_THUMBSTICK_X_PATH: &str = "/user/hand/left/input/thumbstick/x";
pub const LEFT_THUMBSTICK_Y_PATH: &str = "/user/hand/left/input/thumbstick/y";
pub const LEFT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/left/input/thumbstick/click";
pub const LEFT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/left/input/thumbstick/touch";
pub const LEFT_THUMBREST_TOUCH_PATH: &str = "/user/hand/left/input/thumbrest/touch";
pub const RIGHT_SQUEEZE_CLICK_PATH: &str = "/user/hand/right/input/squeeze/click";
pub const RIGHT_SQUEEZE_VALUE_PATH: &str = "/user/hand/right/input/squeeze/value";
pub const RIGHT_TRIGGER_CLICK_PATH: &str = "/user/hand/right/input/trigger/click";
pub const RIGHT_TRIGGER_VALUE_PATH: &str = "/user/hand/right/input/trigger/value";
pub const RIGHT_TRIGGER_TOUCH_PATH: &str = "/user/hand/right/input/trigger/touch";
pub const RIGHT_THUMBSTICK_X_PATH: &str = "/user/hand/right/input/thumbstick/x";
pub const RIGHT_THUMBSTICK_Y_PATH: &str = "/user/hand/right/input/thumbstick/y";
pub const RIGHT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/right/input/thumbstick/click";
pub const RIGHT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/right/input/thumbstick/touch";
pub const RIGHT_THUMBREST_TOUCH_PATH: &str = "/user/hand/right/input/thumbrest/touch";

pub static HEAD_ENTER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_ENTER_CLICK_PATH));
pub static MENU_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(MENU_CLICK_PATH));
pub static A_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(A_CLICK_PATH));
pub static A_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(A_TOUCH_PATH));
pub static B_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(B_CLICK_PATH));
pub static B_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(B_TOUCH_PATH));
pub static X_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(X_CLICK_PATH));
pub static X_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(X_TOUCH_PATH));
pub static Y_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(Y_CLICK_PATH));
pub static Y_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(Y_TOUCH_PATH));
pub static LEFT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_CLICK_PATH));
pub static LEFT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_VALUE_PATH));
pub static LEFT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_CLICK_PATH));
pub static LEFT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_VALUE_PATH));
pub static LEFT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_TOUCH_PATH));
pub static LEFT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_X_PATH));
pub static LEFT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_Y_PATH));
pub static LEFT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_CLICK_PATH));
pub static LEFT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_TOUCH_PATH));
pub static LEFT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBREST_TOUCH_PATH));
pub static RIGHT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_CLICK_PATH));
pub static RIGHT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_VALUE_PATH));
pub static RIGHT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_CLICK_PATH));
pub static RIGHT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_VALUE_PATH));
pub static RIGHT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_TOUCH_PATH));
pub static RIGHT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_X_PATH));
pub static RIGHT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_Y_PATH));
pub static RIGHT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_CLICK_PATH));
pub static RIGHT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_TOUCH_PATH));
pub static RIGHT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBREST_TOUCH_PATH));
