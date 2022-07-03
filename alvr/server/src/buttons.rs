use alvr_common::{once_cell::sync::Lazy, *};
use std::collections::HashMap;

pub static BUTTON_PATH_FROM_ID: Lazy<HashMap<u64, String>> = Lazy::new(|| {
    [
        (*HEAD_ENTER_CLICK_ID, HEAD_ENTER_CLICK_PATH.into()),
        (*MENU_CLICK_ID, MENU_CLICK_PATH.into()),
        (*A_CLICK_ID, A_CLICK_PATH.into()),
        (*A_TOUCH_ID, A_TOUCH_PATH.into()),
        (*B_CLICK_ID, B_CLICK_PATH.into()),
        (*B_TOUCH_ID, B_TOUCH_PATH.into()),
        (*X_CLICK_ID, X_CLICK_PATH.into()),
        (*X_TOUCH_ID, X_TOUCH_PATH.into()),
        (*Y_CLICK_ID, Y_CLICK_PATH.into()),
        (*Y_TOUCH_ID, Y_TOUCH_PATH.into()),
        (*LEFT_SQUEEZE_CLICK_ID, LEFT_SQUEEZE_CLICK_PATH.into()),
        (*LEFT_SQUEEZE_VALUE_ID, LEFT_SQUEEZE_VALUE_PATH.into()),
        (*LEFT_TRIGGER_CLICK_ID, LEFT_TRIGGER_CLICK_PATH.into()),
        (*LEFT_TRIGGER_VALUE_ID, LEFT_TRIGGER_VALUE_PATH.into()),
        (*LEFT_TRIGGER_TOUCH_ID, LEFT_TRIGGER_TOUCH_PATH.into()),
        (*LEFT_THUMBSTICK_X_ID, LEFT_THUMBSTICK_X_PATH.into()),
        (*LEFT_THUMBSTICK_Y_ID, LEFT_THUMBSTICK_Y_PATH.into()),
        (*LEFT_THUMBSTICK_CLICK_ID, LEFT_THUMBSTICK_CLICK_PATH.into()),
        (*LEFT_THUMBSTICK_TOUCH_ID, LEFT_THUMBSTICK_TOUCH_PATH.into()),
        (*LEFT_THUMBREST_TOUCH_ID, LEFT_THUMBREST_TOUCH_PATH.into()),
        (*RIGHT_SQUEEZE_CLICK_ID, RIGHT_SQUEEZE_CLICK_PATH.into()),
        (*RIGHT_SQUEEZE_VALUE_ID, RIGHT_SQUEEZE_VALUE_PATH.into()),
        (*RIGHT_TRIGGER_CLICK_ID, RIGHT_TRIGGER_CLICK_PATH.into()),
        (*RIGHT_TRIGGER_VALUE_ID, RIGHT_TRIGGER_VALUE_PATH.into()),
        (*RIGHT_TRIGGER_TOUCH_ID, RIGHT_TRIGGER_TOUCH_PATH.into()),
        (*RIGHT_THUMBSTICK_X_ID, RIGHT_THUMBSTICK_X_PATH.into()),
        (*RIGHT_THUMBSTICK_Y_ID, RIGHT_THUMBSTICK_Y_PATH.into()),
        (
            *RIGHT_THUMBSTICK_CLICK_ID,
            RIGHT_THUMBSTICK_CLICK_PATH.into(),
        ),
        (
            *RIGHT_THUMBSTICK_TOUCH_ID,
            RIGHT_THUMBSTICK_TOUCH_PATH.into(),
        ),
        (*RIGHT_THUMBREST_TOUCH_ID, RIGHT_THUMBREST_TOUCH_PATH.into()),
    ]
    .into_iter()
    .collect()
});
