mod openxr;

pub use self::openxr::*;

pub enum XrActionType {
    Binary,
    Scalar,
}

pub enum XrActionValue {
    Boolean(bool),
    Scalar(f32),
}

// Note: `tracked` and `has_haptics` should always refer to whether the profile has the
// functionality, not if the funcionality should be enabled/disabled
pub struct XrProfileDesc {
    pub profile: String,
    pub button_bindings: Vec<(String, String)>,
    pub tracked: bool,
    pub has_haptics: bool,
}
