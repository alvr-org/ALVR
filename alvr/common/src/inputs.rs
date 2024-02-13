use crate::hash_string;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

macro_rules! interaction_profile {
    ($ty:ident, $path:expr) => {
        paste::paste! {
            pub const [<$ty _CONTROLLER_PROFILE_PATH>]: &str =
                concat!("/interaction_profiles/", $path, "_controller");
            pub static [<$ty _CONTROLLER_PROFILE_ID>]: Lazy<u64> =
                Lazy::new(|| hash_string([<$ty _CONTROLLER_PROFILE_PATH>]));
        }
    };
}

interaction_profile!(QUEST, "oculus/touch");
interaction_profile!(VIVE, "htc/vive");
interaction_profile!(INDEX, "valve/index");
interaction_profile!(PICO_NEO3, "bytedance/pico_neo3");
interaction_profile!(PICO4, "bytedance/pico4");
interaction_profile!(FOCUS3, "htc/vive_focus3");
interaction_profile!(YVR, "yvr/touch");

macro_rules! devices {
    ($(($name:ident, $path:expr),)*) => {
        paste::paste! {
            $(
                pub const [<$name _PATH>]: &str = $path;
                pub static [<$name _ID>]: Lazy<u64> = Lazy::new(|| hash_string([<$name _PATH>]));
            )*

            pub static DEVICE_ID_TO_PATH: Lazy<HashMap<u64, &str>> = Lazy::new(|| {
                [
                    $((*[<$name _ID>], [<$name _PATH>]),)*
                ]
                .into_iter()
                .collect()
            });
        }
    };
}

devices! {
    (HEAD, "/user/head"),
    (HAND_LEFT, "/user/hand/left"),
    (HAND_RIGHT, "/user/hand/right"),
    (BODY_CHEST, "/user/body/chest"),
    (BODY_HIPS, "/user/body/waist"),
    (BODY_LEFT_ELBOW, "/user/body/left_elbow"),
    (BODY_RIGHT_ELBOW, "/user/body/right_elbow"),
    (BODY_LEFT_KNEE, "/user/body/left_knee"),
    (BODY_LEFT_FOOT, "/user/body/left_foot"),
    (BODY_RIGHT_KNEE, "/user/body/right_knee"),
    (BODY_RIGHT_FOOT, "/user/body/right_foot"),
}

pub enum ButtonType {
    Binary,
    Scalar,
}

pub struct ButtonInfo {
    pub path: &'static str,
    pub button_type: ButtonType,
    pub device_id: u64,
}

macro_rules! controller_inputs {
    ($(($inputs:ident, $paths:literal, $ty:ident),)*) => {
        paste::paste! {
            $(
                pub const [<LEFT_ $inputs _PATH>]: &str =
                    concat!("/user/hand/left/input/", $paths);
                pub static [<LEFT_ $inputs _ID>]: Lazy<u64> =
                    Lazy::new(|| hash_string([<LEFT_ $inputs _PATH>]));
                pub const [<RIGHT_ $inputs _PATH>]: &str =
                    concat!("/user/hand/right/input/", $paths);
                pub static [<RIGHT_ $inputs _ID>]: Lazy<u64> =
                    Lazy::new(|| hash_string([<RIGHT_ $inputs _PATH>]));
            )*

            pub static BUTTON_INFO: Lazy<HashMap<u64, ButtonInfo>> = Lazy::new(|| {
                [
                    $((
                        *[<LEFT_ $inputs _ID>],
                        ButtonInfo {
                            path: [<LEFT_ $inputs _PATH>],
                            button_type: ButtonType::$ty,
                            device_id: *HAND_LEFT_ID,
                        },
                    ),
                    (
                        *[<RIGHT_ $inputs _ID>],
                        ButtonInfo {
                            path: [<RIGHT_ $inputs _PATH>],
                            button_type: ButtonType::$ty,
                            device_id: *HAND_RIGHT_ID,
                        },
                    ),)*
                ]
                .into_iter()
                .collect()
            });
        }
    };
}

controller_inputs! {
    (SYSTEM_CLICK, "system/click", Binary),
    (SYSTEM_TOUCH, "system/touch", Binary),
    (MENU_CLICK, "menu/click", Binary),
    (BACK_CLICK, "back/click", Binary),
    (A_CLICK, "a/click", Binary),
    (A_TOUCH, "a/touch", Binary),
    (B_CLICK, "b/click", Binary),
    (B_TOUCH, "b/touch", Binary),
    (X_CLICK, "x/click", Binary),
    (X_TOUCH, "x/touch", Binary),
    (Y_CLICK, "y/click", Binary),
    (Y_TOUCH, "y/touch", Binary),
    (SQUEEZE_CLICK, "squeeze/click", Binary),
    (SQUEEZE_TOUCH, "squeeze/touch", Binary),
    (SQUEEZE_VALUE, "squeeze/value", Scalar),
    (SQUEEZE_FORCE, "squeeze/force", Scalar),
    (TRIGGER_CLICK, "trigger/click", Binary),
    (TRIGGER_VALUE, "trigger/value", Scalar),
    (TRIGGER_TOUCH, "trigger/touch", Binary),
    (THUMBSTICK_X, "thumbstick/x", Scalar),
    (THUMBSTICK_Y, "thumbstick/y", Scalar),
    (THUMBSTICK_CLICK, "thumbstick/click", Binary),
    (THUMBSTICK_TOUCH, "thumbstick/touch", Binary),
    (TRACKPAD_X, "trackpad/x", Scalar),
    (TRACKPAD_Y, "trackpad/y", Scalar),
    (TRACKPAD_CLICK, "trackpad/click", Binary),
    (TRACKPAD_FORCE, "trackpad/force", Scalar),
    (TRACKPAD_TOUCH, "trackpad/touch", Binary),
    (THUMBREST_TOUCH, "thumbrest/touch", Binary),
}

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
