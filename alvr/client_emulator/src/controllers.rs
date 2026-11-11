//! Emulated controller state and controller profiles.
//!
//! Controller poses are head-relative, in ALVR's coordinate convention: X right, Y up, -Z forward.
//! The design document describes a Z-up head frame; the axes here are deliberately the backend's
//! instead, as the document allows, so no conversion exists anywhere in the pipeline. A pose from
//! the design's frame maps as `(x, y, z) -> (x, z_design_up -> y, forward y_design -> -z)`.
//!
//! Profiles mirror `alvr_common::CONTROLLER_PROFILE_INFO`: each lists the inputs ALVR would accept
//! from that controller in real life. They are loaded from a user-editable JSON file next to the
//! executable so new controllers can be described without recompiling; the default file is
//! generated from the ALVR definitions themselves, which keeps the two from drifting apart.

use alvr_common::{
    FOCUS3_CONTROLLER_PROFILE_PATH, INDEX_CONTROLLER_PROFILE_PATH, PICO_G3_CONTROLLER_PROFILE_PATH,
    PICO_NEO3_CONTROLLER_PROFILE_PATH, PICO4_CONTROLLER_PROFILE_PATH,
    PICO4S_CONTROLLER_PROFILE_PATH, PSVR2_CONTROLLER_PROFILE_PATH, Pose,
    QUEST_CONTROLLER_PROFILE_PATH, VIVE_CONTROLLER_PROFILE_PATH, YVR_CONTROLLER_PROFILE_PATH,
    anyhow::{Context, Result},
    glam::{EulerRot, Quat, Vec3},
    hash_string, info, warn,
};
use alvr_packets::ButtonValue;
use alvr_system_info::Platform;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// Name of the controller settings file loaded from the executable's directory.
pub const SETTINGS_FILE_NAME: &str = "controllers.json";

/// Which hand a controller belongs to. Doubles as the index into per-hand arrays.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Left = 0,
    Right = 1,
}

impl Hand {
    pub const BOTH: [Hand; 2] = [Hand::Left, Hand::Right];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Lower-case name as used in input paths and API URLs.
    pub fn side(self) -> &'static str {
        match self {
            Hand::Left => "left",
            Hand::Right => "right",
        }
    }

    pub fn from_side(side: &str) -> Option<Self> {
        match side {
            "left" => Some(Hand::Left),
            "right" => Some(Hand::Right),
            _ => None,
        }
    }
}

/// Every input path suffix ALVR understands, in the order inputs are listed in settings files and
/// documentation. Matches the declaration order of `alvr_common::inputs`.
pub const INPUT_SUFFIXES: &[&str] = &[
    "system/click",
    "system/touch",
    "menu/click",
    "menu/touch",
    "back/click",
    "a/click",
    "a/touch",
    "b/click",
    "b/touch",
    "x/click",
    "x/touch",
    "y/click",
    "y/touch",
    "squeeze/click",
    "squeeze/touch",
    "squeeze/value",
    "squeeze/force",
    "squeeze/sensor/value",
    "trigger/click",
    "trigger/value",
    "trigger/touch",
    "trigger/sensor/value",
    "thumbstick/x",
    "thumbstick/y",
    "thumbstick/click",
    "thumbstick/touch",
    "trackpad/x",
    "trackpad/y",
    "trackpad/click",
    "trackpad/force",
    "trackpad/touch",
    "thumbrest/touch",
];

/// Display names for the profiles ALVR predefines, in dropdown order.
const PROFILE_NAMES: &[(&str, &str)] = &[
    (QUEST_CONTROLLER_PROFILE_PATH, "Quest"),
    (INDEX_CONTROLLER_PROFILE_PATH, "Index"),
    (VIVE_CONTROLLER_PROFILE_PATH, "Vive Wand"),
    (PICO_NEO3_CONTROLLER_PROFILE_PATH, "Pico Neo3"),
    (PICO4_CONTROLLER_PROFILE_PATH, "Pico 4"),
    (PICO4S_CONTROLLER_PROFILE_PATH, "Pico 4S"),
    (PICO_G3_CONTROLLER_PROFILE_PATH, "Pico G3"),
    (PSVR2_CONTROLLER_PROFILE_PATH, "PSVR2 Sense"),
    (FOCUS3_CONTROLLER_PROFILE_PATH, "Vive Focus 3"),
    (YVR_CONTROLLER_PROFILE_PATH, "YVR"),
];

/// Full input path for a hand and suffix, e.g. `/user/hand/left/input/trigger/value`.
pub fn input_path(hand: Hand, suffix: &str) -> String {
    format!("/user/hand/{}/input/{}", hand.side(), suffix)
}

/// Whether an input suffix is a scalar (axis) rather than a binary (button), per ALVR's own
/// definitions. `None` when the suffix is not one ALVR knows.
pub fn input_is_scalar(suffix: &str) -> Option<bool> {
    // Left and right entries always have the same type, so looking up one hand is enough.
    let id = hash_string(&input_path(Hand::Left, suffix));

    alvr_common::BUTTON_INFO
        .get(&id)
        .map(|info| matches!(info.button_type, alvr_common::ButtonType::Scalar))
}

/// One input a profile supports, with its hashed path id precomputed.
#[derive(Clone)]
pub struct ProfileInput {
    /// Path suffix such as `trigger/value`. Interned so state maps can key on `&'static str`.
    pub suffix: &'static str,
    pub id: u64,
    pub scalar: bool,
}

/// A controller type the emulator can present to the server.
pub struct Profile {
    pub name: String,
    /// Interaction profile path, e.g. `/interaction_profiles/oculus/touch_controller`.
    pub path: String,
    pub id: u64,
    /// Supported inputs per hand, in canonical order. Left is index 0.
    pub inputs: [Vec<ProfileInput>; 2],
    /// Optional glTF models shown when the controller is set visible, one per hand since the
    /// physical pair is mirrored, authored around the grip. Left is index 0.
    pub models: [Option<PathBuf>; 2],
    /// The grip pose in the aim pose's frame, per hand. Left is index 0.
    ///
    /// The emulator's controller state is the *aim* — where the controller points, which is what
    /// the pads steer — but ALVR is sent the *grip*, as a real client sends it. The two differ by
    /// the controller's shape: a Quest handle is tilted 60° up from where it points. Sending the
    /// aim as though it were the grip tips the application's pointer away by that angle, and
    /// rolling the controller then swings it off to the side.
    pub grip_from_aim: [Pose; 2],
    /// What ALVR is actually sent, in the grip pose's frame, per hand. Left is index 0.
    ///
    /// A real client does not send the runtime's grip as it is: it first applies a per-headset
    /// correction ([`alvr_client_core::controller_pose_offset`]) that brings the grip into line
    /// with what SteamVR's controller models expect. The emulator reports itself as a desktop, so
    /// it applies the correction of the headset the profile's controller belongs to.
    pub sent_from_grip: [Pose; 2],
}

impl Profile {
    pub fn supports(&self, hand: Hand, suffix: &str) -> bool {
        self.inputs[hand.index()]
            .iter()
            .any(|input| input.suffix == suffix)
    }

    pub fn input(&self, hand: Hand, suffix: &str) -> Option<&ProfileInput> {
        self.inputs[hand.index()]
            .iter()
            .find(|input| input.suffix == suffix)
    }

    /// All input path ids of both hands, as announced to the server via the active interaction
    /// profile packet.
    pub fn input_id_set(&self) -> HashSet<u64> {
        self.inputs.iter().flatten().map(|input| input.id).collect()
    }
}

/// Controller emulation settings, resolved from the settings file.
pub struct ControllerSettings {
    /// Radians of controller rotation per pixel of drag on the rotation pads.
    pub rotation_sensitivity: f32,
    /// Head-relative starting position per hand. Left is index 0.
    pub start_positions: [Vec3; 2],
    /// Head-relative starting orientation of the aim, from the configured starting pitch.
    pub start_orientation: Quat,
    pub profiles: Vec<Profile>,
}

/// On-disk form of [`ControllerSettings`]. Kept separate so the file format stays plain data.
#[derive(Serialize, Deserialize)]
struct SettingsFile {
    rotation_sensitivity: f32,
    left_start_position: [f32; 3],
    right_start_position: [f32; 3],
    /// Upward pitch of the resting aim, in degrees — the direction the pointer leaves in. Aimed
    /// above the horizon on purpose: pointed level or below, the controllers' laser rests on
    /// SteamVR's status panel, whose system UI then captures input focus and replaces the
    /// application's controller rendering with the laggy web-based ghost silhouettes.
    #[serde(default = "default_start_pitch")]
    start_pitch_degrees: f32,
    profiles: Vec<ProfileEntry>,
}

/// Was 30° while the controller state was sent as the grip, which tilted the Quest's pointer about
/// 40° below it; that pitch only cancelled the tilt. Now the state is the aim itself.
fn default_start_pitch() -> f32 {
    10.0
}

#[derive(Serialize, Deserialize)]
struct ProfileEntry {
    name: String,
    path: String,
    left_inputs: Vec<String>,
    right_inputs: Vec<String>,
    /// glTF models authored around the grip, relative to the executable's directory. A file
    /// without one gets the bundled model for its profile path, if there is one, as with
    /// `grip_offset`; an empty string asks for the placeholder instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    left_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    right_model: Option<String>,
    /// Where the grip sits relative to the aim. A file without one gets the built-in offset for
    /// its profile path, if there is one, so settings files written before it existed still
    /// work; `null` there means the grip and aim coincide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grip_offset: Option<GripOffset>,
}

/// The grip pose relative to the aim pose, for the left controller. The right controller is its
/// mirror image, mirrored the way ALVR's server mirrors its own controller offsets: across X.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct GripOffset {
    /// Metres, in the aim pose's frame.
    position: [f32; 3],
    /// Degrees about the aim pose's X, Y and Z axes, applied in that order.
    rotation_degrees: [f32; 3],
}

impl GripOffset {
    fn pose(self, hand: Hand) -> Pose {
        let [x, y, z] = self.rotation_degrees.map(f32::to_radians);
        let [px, py, pz] = self.position;

        match hand {
            Hand::Left => Pose {
                orientation: Quat::from_euler(EulerRot::XYZ, x, y, z),
                position: Vec3::new(px, py, pz),
            },
            Hand::Right => Pose {
                orientation: Quat::from_euler(EulerRot::XYZ, x, -y, -z),
                position: Vec3::new(-px, py, pz),
            },
        }
    }
}

/// The built-in grip offset for a profile, where one is known.
///
/// The Quest's comes from SteamVR's own model of the Quest 2 controller
/// (`resources/rendermodels/oculus_quest2_controller_left`), whose `openxr_grip` and `openxr_aim`
/// components place the two OpenXR poses against the controller: the grip 20.6° up from the
/// device's axis in the handle, the aim 39.4° down from it at the ring. Composed, the grip is 60°
/// up from the aim, 3 cm below it and 9.5 cm behind it. Other controllers have no known offset yet
/// and send the aim unchanged, as before.
fn default_grip_offset(path: &str) -> Option<GripOffset> {
    (path == QUEST_CONTROLLER_PROFILE_PATH).then_some(GripOffset {
        position: [0.0, -0.030, 0.095],
        rotation_degrees: [60.0, 0.0, 0.0],
    })
}

/// The bundled model for a profile, where there is one: the Quest 2 controllers from the WebXR
/// Input Profiles, aligned to the emulator's grip by `tools/fetch_controller_model.py`.
fn default_model(path: &str, hand: Hand) -> Option<String> {
    (path == QUEST_CONTROLLER_PROFILE_PATH)
        .then(|| format!("models/quest_controller_{}.glb", hand.side()))
}

/// The headset a profile's controller ships with, whose client-side pose correction is applied.
/// Only profiles with a known grip offset name one, since the correction is relative to the grip.
fn headset_platform(path: &str) -> Option<Platform> {
    (path == QUEST_CONTROLLER_PROFILE_PATH).then_some(Platform::Quest2)
}

impl ControllerSettings {
    /// Loads the settings file in `directory`, writing the default one first if none
    /// exists. A file that cannot be parsed falls back to the defaults without being overwritten,
    /// so a user's editing mistake never destroys their file.
    ///
    /// Relative model paths in it are resolved against `models_dir`.
    pub fn load_or_create(directory: &Path, models_dir: &Path) -> Self {
        let path = directory.join(SETTINGS_FILE_NAME);

        if !path.exists() {
            match serde_json::to_string_pretty(&default_file()) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&path, json + "\n") {
                        warn!("Cannot write default controller settings: {e}");
                    } else {
                        info!("Wrote default controller settings to {}", path.display());
                    }
                }
                Err(e) => warn!("Cannot serialise default controller settings: {e}"),
            }
        }

        let file = match load_file(&path) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    "Cannot load {}: {e:#}. Using built-in controller settings.",
                    path.display()
                );
                default_file()
            }
        };

        let mut settings = resolve(file, models_dir);

        // Everything indexes into the profile list, so it must never be empty.
        if settings.profiles.is_empty() {
            warn!("Controller settings contain no profiles; using the built-in ones");
            settings.profiles = resolve(default_file(), models_dir).profiles;
        }

        settings
    }

    /// Finds a profile by display name (case-insensitive) or interaction profile path.
    pub fn find_profile(&self, name: &str) -> Option<usize> {
        self.profiles
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name) || p.path == name)
    }
}

fn load_file(path: &Path) -> Result<SettingsFile> {
    let text = std::fs::read_to_string(path).context("Cannot read controller settings")?;
    serde_json::from_str(&text).context("Cannot parse controller settings")
}

/// The default settings file, generated from ALVR's own interaction profile definitions so the
/// emulated input sets are exactly what ALVR accepts from each controller.
fn default_file() -> SettingsFile {
    let profiles = PROFILE_NAMES
        .iter()
        .filter_map(|(path, name)| {
            let button_set = &alvr_common::CONTROLLER_PROFILE_INFO
                .get(&hash_string(path))?
                .button_set;

            let inputs_for = |hand: Hand| {
                INPUT_SUFFIXES
                    .iter()
                    .filter(|suffix| button_set.contains(&hash_string(&input_path(hand, suffix))))
                    .map(|suffix| (*suffix).to_owned())
                    .collect()
            };

            Some(ProfileEntry {
                name: (*name).to_owned(),
                path: (*path).to_owned(),
                left_inputs: inputs_for(Hand::Left),
                right_inputs: inputs_for(Hand::Right),
                left_model: default_model(path, Hand::Left),
                right_model: default_model(path, Hand::Right),
                grip_offset: default_grip_offset(path),
            })
        })
        .collect();

    SettingsFile {
        rotation_sensitivity: 0.005,
        left_start_position: [-0.15, -0.25, -0.35],
        right_start_position: [0.15, -0.25, -0.35],
        start_pitch_degrees: default_start_pitch(),
        profiles,
    }
}

fn resolve(file: SettingsFile, directory: &Path) -> ControllerSettings {
    let profiles = file
        .profiles
        .into_iter()
        .map(|entry| {
            let resolve_inputs = |hand: Hand, suffixes: &[String]| {
                suffixes
                    .iter()
                    .filter_map(|suffix| {
                        // Interning against the known list also rejects inputs ALVR cannot carry,
                        // which a custom profile might mistakenly list.
                        let Some(interned) = INPUT_SUFFIXES.iter().find(|known| **known == *suffix)
                        else {
                            warn!(
                                "Profile '{}' lists unknown input '{suffix}'; ignoring it",
                                entry.name
                            );
                            return None;
                        };

                        Some(ProfileInput {
                            suffix: interned,
                            id: hash_string(&input_path(hand, interned)),
                            scalar: input_is_scalar(interned).unwrap_or(false),
                        })
                    })
                    .collect()
            };

            let grip_offset = entry
                .grip_offset
                .or_else(|| default_grip_offset(&entry.path));
            let grip_from_aim =
                |hand: Hand| grip_offset.map_or(Pose::IDENTITY, |offset| offset.pose(hand));
            let platform = headset_platform(&entry.path);
            let sent_from_grip = |hand: Hand| {
                platform.map_or(Pose::IDENTITY, |platform| {
                    alvr_client_core::controller_pose_offset(platform, hand == Hand::Right)
                })
            };
            let model = |configured: &Option<String>, hand: Hand| {
                configured
                    .clone()
                    .or_else(|| default_model(&entry.path, hand))
                    .filter(|model| !model.is_empty())
                    .map(|model| directory.join(model))
            };

            Profile {
                id: hash_string(&entry.path),
                inputs: [
                    resolve_inputs(Hand::Left, &entry.left_inputs),
                    resolve_inputs(Hand::Right, &entry.right_inputs),
                ],
                models: [
                    model(&entry.left_model, Hand::Left),
                    model(&entry.right_model, Hand::Right),
                ],
                grip_from_aim: [grip_from_aim(Hand::Left), grip_from_aim(Hand::Right)],
                sent_from_grip: [sent_from_grip(Hand::Left), sent_from_grip(Hand::Right)],
                name: entry.name,
                path: entry.path,
            }
        })
        .collect();

    ControllerSettings {
        rotation_sensitivity: file.rotation_sensitivity,
        start_positions: [
            Vec3::from_array(file.left_start_position),
            Vec3::from_array(file.right_start_position),
        ],
        start_orientation: Quat::from_rotation_x(file.start_pitch_degrees.to_radians()),
        profiles,
    }
}

/// The controller poses the toolbar can draw as an axis gizmo: the two OpenXR poses, and what ALVR
/// is actually sent, since that last step is where a controller is most easily off.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ControllerInteraction {
    /// `/input/aim/pose`, which the pads steer.
    Aim,
    /// `/input/grip/pose`, derived from the aim by the profile's grip offset.
    Grip,
    /// The grip with the headset's client-side correction applied, as it goes on the wire.
    Sent,
}

impl ControllerInteraction {
    pub const ALL: [Self; 3] = [Self::Aim, Self::Grip, Self::Sent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Aim => "Aim",
            Self::Grip => "Grip",
            Self::Sent => "Sent",
        }
    }

    /// Shown as the dropdown entry's tooltip.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Aim => {
                "Where the controller points, and where the application's pointer should start: \
                 -Z forward. This is what the pads and the API steer; the orange nose of the \
                 placeholder shows it too."
            }
            Self::Grip => {
                "How the hand holds the controller: on the axis of the handle, -Z running up it \
                 towards the index finger. Derived from the aim by the profile's grip_offset."
            }
            Self::Sent => {
                "What ALVR is sent: the grip with the headset's client-side correction, as a real \
                 client sends it. The server moves this forward by its controller offset to place \
                 SteamVR's device."
            }
        }
    }

    /// Where this pose sits, head-relative, for the given controller.
    pub fn pose(self, state: &ControllerState, profile: Option<&Profile>, hand: Hand) -> Pose {
        match self {
            Self::Aim => state.aim_pose(),
            Self::Grip => state.grip_pose(profile, hand),
            Self::Sent => state.sent_pose(profile, hand),
        }
    }
}

/// Live state of one emulated controller. Owned by the UI thread; the HTTP API mutates it through
/// queued commands, so user input and API input merge in one place.
pub struct ControllerState {
    pub enabled: bool,
    pub profile_index: usize,
    /// Show the 3D model in the scene view.
    pub model_visible: bool,
    /// Head-relative position of the aim pose. See the module docs for the axes.
    pub position: Vec3,
    /// Head-relative orientation of the aim pose: -Z is where the controller points. The grip
    /// sent to ALVR is derived from it; see [`Profile::grip_from_aim`].
    pub orientation: Quat,
    /// Explicitly held inputs, keyed by input path suffix. Values equal to the resting state are
    /// removed rather than stored, so this map is exactly the set of active inputs.
    pub inputs: HashMap<&'static str, ButtonValue>,
}

impl ControllerState {
    pub fn new(settings: &ControllerSettings, hand: Hand) -> Self {
        Self {
            enabled: false,
            profile_index: 0,
            model_visible: false,
            position: settings.start_positions[hand.index()],
            orientation: settings.start_orientation,
            inputs: HashMap::new(),
        }
    }

    /// The head-relative aim pose this state holds.
    pub fn aim_pose(&self) -> Pose {
        Pose {
            orientation: self.orientation.normalize(),
            position: self.position,
        }
    }

    /// The head-relative grip pose for this aim, under the given profile.
    pub fn grip_pose(&self, profile: Option<&Profile>, hand: Hand) -> Pose {
        let offset = profile.map_or(Pose::IDENTITY, |profile| {
            profile.grip_from_aim[hand.index()]
        });

        self.aim_pose() * offset
    }

    /// The head-relative pose ALVR is sent for this aim: the grip with the headset's client-side
    /// correction applied, as a real client sends it.
    pub fn sent_pose(&self, profile: Option<&Profile>, hand: Hand) -> Pose {
        let offset = profile.map_or(Pose::IDENTITY, |profile| {
            profile.sent_from_grip[hand.index()]
        });

        self.grip_pose(profile, hand) * offset
    }

    /// Returns the pose and inputs to their defaults. Emulation stays enabled and the profile
    /// selection is kept, since those are configuration rather than state.
    pub fn reset(&mut self, settings: &ControllerSettings, hand: Hand) {
        self.position = settings.start_positions[hand.index()];
        self.orientation = settings.start_orientation;
        self.inputs.clear();
    }

    /// Sets one input, clamping scalars to their valid range and dropping values equal to the
    /// resting state so releases are represented by absence.
    pub fn set_input(&mut self, suffix: &'static str, value: ButtonValue) {
        let value = match value {
            ButtonValue::Scalar(scalar) => {
                // Stick and trackpad axes are signed; everything else runs 0 to 1.
                let signed = suffix.ends_with("/x") || suffix.ends_with("/y");
                let min = if signed { -1.0 } else { 0.0 };
                ButtonValue::Scalar(scalar.clamp(min, 1.0))
            }
            binary => binary,
        };

        let resting = match value {
            ButtonValue::Binary(pressed) => !pressed,
            ButtonValue::Scalar(scalar) => scalar == 0.0,
        };

        if resting {
            self.inputs.remove(suffix);
        } else {
            self.inputs.insert(suffix, value);
        }
    }

    pub fn scalar(&self, suffix: &str) -> f32 {
        match self.inputs.get(suffix) {
            Some(ButtonValue::Scalar(value)) => *value,
            Some(ButtonValue::Binary(true)) => 1.0,
            _ => 0.0,
        }
    }

    pub fn binary(&self, suffix: &str) -> bool {
        match self.inputs.get(suffix) {
            Some(ButtonValue::Binary(value)) => *value,
            Some(ButtonValue::Scalar(value)) => *value > 0.0,
            _ => false,
        }
    }

    /// The button entries this controller currently produces, filtered to what the profile
    /// supports and completed with the inputs a physical controller would report alongside the
    /// explicit ones: touching follows from pressing, a full digital press implies a full analog
    /// value, and so on. Explicit values always win over derived ones.
    pub fn effective_entries(&self, profile: &Profile, hand: Hand) -> Vec<(u64, ButtonValue)> {
        let mut values: HashMap<&str, ButtonValue> = HashMap::new();

        for (suffix, value) in &self.inputs {
            if profile.supports(hand, suffix) {
                values.insert(suffix, *value);
            }
        }

        let mut derive = |suffix: &'static str, value: ButtonValue| {
            if profile.supports(hand, suffix) {
                values.entry(suffix).or_insert(value);
            }
        };

        // Analog pulls: a digital press stands in for a full pull, and any pull implies contact.
        for control in ["trigger", "squeeze"] {
            let click_suffix = suffix_of(control, "click");
            let value_suffix = suffix_of(control, "value");
            let touch_suffix = suffix_of(control, "touch");

            let value = self.scalar(value_suffix);
            let clicked = self.binary(click_suffix);

            if clicked {
                derive(value_suffix, ButtonValue::Scalar(1.0));
            }
            if value >= FULL_PULL_THRESHOLD {
                derive(click_suffix, ButtonValue::Binary(true));
            }
            if clicked || value > 0.0 {
                derive(touch_suffix, ButtonValue::Binary(true));
            }
        }

        // Sticks and pads: deflection or a click implies the thumb resting on them.
        for control in ["thumbstick", "trackpad"] {
            let deflected = self.scalar(suffix_of(control, "x")) != 0.0
                || self.scalar(suffix_of(control, "y")) != 0.0;

            if deflected || self.binary(suffix_of(control, "click")) {
                derive(suffix_of(control, "touch"), ButtonValue::Binary(true));
            }
        }

        // Face and system buttons: a press implies a touch.
        for control in ["a", "b", "x", "y", "menu", "system"] {
            if self.binary(suffix_of(control, "click")) {
                derive(suffix_of(control, "touch"), ButtonValue::Binary(true));
            }
        }

        values
            .into_iter()
            .filter_map(|(suffix, value)| {
                profile
                    .input(hand, suffix)
                    .map(|input| (input.id, coerce(value, input.scalar)))
            })
            .collect()
    }
}

/// A trigger or grip pulled this far also reports its click, mirroring physical controllers where
/// the click engages just before the end of travel.
const FULL_PULL_THRESHOLD: f32 = 0.9;

/// Interned `control/action` suffix, so derived lookups reuse the canonical strings.
fn suffix_of(control: &str, action: &str) -> &'static str {
    INPUT_SUFFIXES
        .iter()
        .find(|suffix| {
            suffix
                .strip_prefix(control)
                .and_then(|rest| rest.strip_prefix('/'))
                .is_some_and(|rest| rest == action)
        })
        .copied()
        .unwrap_or("")
}

/// Aligns a value with the input's declared type, so a binary set through a scalar input path (or
/// vice versa) still produces a well-typed entry.
fn coerce(value: ButtonValue, scalar: bool) -> ButtonValue {
    match (value, scalar) {
        (ButtonValue::Binary(pressed), true) => {
            ButtonValue::Scalar(if pressed { 1.0 } else { 0.0 })
        }
        (ButtonValue::Scalar(value), false) => ButtonValue::Binary(value > 0.5),
        (value, _) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SteamVR's Quest 2 controller model places the OpenXR grip and aim against the device pose
    /// (`openxr_grip` and `openxr_aim` in `oculus_quest2_controller_{left,right}.json`). The
    /// built-in offset must be the grip as seen from the aim, for both hands, or the pointer an
    /// application draws leaves the controller at an angle to where it is being aimed.
    #[test]
    fn quest_grip_offset_matches_steamvr_model() {
        let component = |x: f32, origin: [f32; 3], pitch_degrees: f32| Pose {
            orientation: Quat::from_rotation_x(pitch_degrees.to_radians()),
            position: Vec3::new(x, origin[1], origin[2]),
        };

        let settings = resolve(default_file(), Path::new("."));
        let quest = &settings.profiles[settings.find_profile("Quest").expect("Quest profile")];

        for (hand, x) in [(Hand::Left, 0.007), (Hand::Right, -0.007)] {
            let grip = component(x, [0.0, -0.001_829_41, 0.101_948_2], 20.6);
            let aim = component(x, [0.0, -0.038_947_66, 0.009_496_94], -39.4);
            let expected = aim.inverse() * grip;
            let actual = quest.grip_from_aim[hand.index()];

            let position_error = actual.position.distance(expected.position);
            let angle_error = actual.orientation.angle_between(expected.orientation);

            assert!(
                position_error < 0.001,
                "{} grip offset is {:.1} mm off SteamVR's model",
                hand.side(),
                position_error * 1000.0
            );
            assert!(
                angle_error.to_degrees() < 0.1,
                "{} grip offset is {:.2}° off SteamVR's model",
                hand.side(),
                angle_error.to_degrees()
            );
        }
    }

    /// Follows a Quest aim all the way to SteamVR's pointer: the emulator sends the corrected grip,
    /// the server moves it 11 cm forward to the device pose (its default controller offset), and
    /// SteamVR's model puts its `openxr_aim` there. The pointer should leave where the emulator
    /// aims, within the few degrees the real client's Quest correction is known to be off (it
    /// rotates the grip by 15°, where SteamVR's model implies 20.6°).
    #[test]
    fn quest_pointer_follows_the_aim() {
        let settings = resolve(default_file(), Path::new("."));
        let profile_index = settings.find_profile("Quest").expect("Quest profile");
        let quest = &settings.profiles[profile_index];

        for (hand, x) in [(Hand::Left, 0.007), (Hand::Right, -0.007)] {
            let mut state = ControllerState::new(&settings, hand);
            state.profile_index = profile_index;
            state.position = Vec3::ZERO;
            state.orientation = Quat::IDENTITY;

            let device = state.sent_pose(Some(quest), hand)
                * Pose {
                    orientation: Quat::IDENTITY,
                    position: Vec3::new(0.0, 0.0, -0.11),
                };
            let steamvr_aim = device
                * Pose {
                    orientation: Quat::from_rotation_x((-39.4f32).to_radians()),
                    position: Vec3::new(x, -0.038_947_66, 0.009_496_94),
                };

            let pointer = steamvr_aim.orientation * Vec3::NEG_Z;
            let error = pointer.angle_between(Vec3::NEG_Z).to_degrees();
            println!(
                "{}: pointer {error:.1}° off the aim, origin {:.1} cm away",
                hand.side(),
                steamvr_aim.position.length() * 100.0
            );

            assert!(
                error < 6.0,
                "{} pointer is {error:.1}° off the aim",
                hand.side()
            );
            assert!(
                pointer.y > 0.0,
                "{} pointer should err upwards, by the client correction's shortfall",
                hand.side()
            );
        }
    }

    /// A settings file written before the bundled models existed names none, and still gets them;
    /// an empty string is how to ask for the placeholder instead.
    #[test]
    fn quest_gets_the_bundled_model_unless_asked_not_to() {
        let mut file = default_file();
        let quest = file
            .profiles
            .iter_mut()
            .find(|entry| entry.path == QUEST_CONTROLLER_PROFILE_PATH)
            .expect("Quest profile");
        quest.left_model = None;
        quest.right_model = Some(String::new());

        let settings = resolve(file, Path::new("."));
        let quest = &settings.profiles[settings.find_profile("Quest").expect("Quest profile")];

        assert_eq!(
            quest.models[0].as_deref(),
            Some(Path::new("./models/quest_controller_left.glb"))
        );
        assert_eq!(quest.models[1], None);
    }

    /// A settings file written before grip offsets existed has none, and must still get the
    /// built-in one for its profile rather than silently sending the aim as the grip.
    #[test]
    fn older_settings_files_get_the_built_in_offset() {
        let mut file = default_file();
        for entry in &mut file.profiles {
            entry.grip_offset = None;
        }

        let settings = resolve(file, Path::new("."));
        let quest = &settings.profiles[settings.find_profile("Quest").expect("Quest profile")];

        assert!(
            quest.grip_from_aim[0]
                .orientation
                .angle_between(Quat::IDENTITY)
                > 1.0,
            "the Quest grip offset was not filled in"
        );
    }
}
