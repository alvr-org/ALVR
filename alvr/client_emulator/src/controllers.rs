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
    FOCUS3_CONTROLLER_PROFILE_PATH, INDEX_CONTROLLER_PROFILE_PATH,
    PICO4_CONTROLLER_PROFILE_PATH, PICO4S_CONTROLLER_PROFILE_PATH,
    PICO_G3_CONTROLLER_PROFILE_PATH, PICO_NEO3_CONTROLLER_PROFILE_PATH,
    PSVR2_CONTROLLER_PROFILE_PATH, QUEST_CONTROLLER_PROFILE_PATH, VIVE_CONTROLLER_PROFILE_PATH,
    YVR_CONTROLLER_PROFILE_PATH,
    anyhow::{Context, Result},
    glam::{Quat, Vec3},
    hash_string, info, warn,
};
use alvr_packets::ButtonValue;
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

    alvr_common::BUTTON_INFO.get(&id).map(|info| {
        matches!(info.button_type, alvr_common::ButtonType::Scalar)
    })
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
    /// physical pair is mirrored. Left is index 0.
    pub models: [Option<PathBuf>; 2],
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
        self.inputs
            .iter()
            .flatten()
            .map(|input| input.id)
            .collect()
    }
}

/// Controller emulation settings, resolved from the settings file.
pub struct ControllerSettings {
    /// Radians of controller rotation per pixel of drag on the rotation pads.
    pub rotation_sensitivity: f32,
    /// Head-relative starting position per hand. Left is index 0.
    pub start_positions: [Vec3; 2],
    pub profiles: Vec<Profile>,
}

/// On-disk form of [`ControllerSettings`]. Kept separate so the file format stays plain data.
#[derive(Serialize, Deserialize)]
struct SettingsFile {
    rotation_sensitivity: f32,
    left_start_position: [f32; 3],
    right_start_position: [f32; 3],
    profiles: Vec<ProfileEntry>,
}

#[derive(Serialize, Deserialize)]
struct ProfileEntry {
    name: String,
    path: String,
    left_inputs: Vec<String>,
    right_inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    left_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    right_model: Option<String>,
}

impl ControllerSettings {
    /// Loads the settings file next to the executable, writing the default one first if none
    /// exists. A file that cannot be parsed falls back to the defaults without being overwritten,
    /// so a user's editing mistake never destroys their file.
    pub fn load_or_create(directory: &Path) -> Self {
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

        let mut settings = resolve(file, directory);

        // Everything indexes into the profile list, so it must never be empty.
        if settings.profiles.is_empty() {
            warn!("Controller settings contain no profiles; using the built-in ones");
            settings.profiles = resolve(default_file(), directory).profiles;
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
                left_model: None,
                right_model: None,
            })
        })
        .collect();

    SettingsFile {
        rotation_sensitivity: 0.005,
        left_start_position: [-0.15, -0.25, -0.35],
        right_start_position: [0.15, -0.25, -0.35],
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
                        let Some(interned) =
                            INPUT_SUFFIXES.iter().find(|known| **known == *suffix)
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

            Profile {
                id: hash_string(&entry.path),
                inputs: [
                    resolve_inputs(Hand::Left, &entry.left_inputs),
                    resolve_inputs(Hand::Right, &entry.right_inputs),
                ],
                models: [
                    entry.left_model.as_ref().map(|model| directory.join(model)),
                    entry.right_model.as_ref().map(|model| directory.join(model)),
                ],
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
        profiles,
    }
}

/// Live state of one emulated controller. Owned by the UI thread; the HTTP API mutates it through
/// queued commands, so user input and API input merge in one place.
pub struct ControllerState {
    pub enabled: bool,
    pub profile_index: usize,
    /// Show the 3D model in the scene view.
    pub model_visible: bool,
    /// Head-relative position. See the module docs for the axes.
    pub position: Vec3,
    /// Head-relative orientation.
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
            orientation: Quat::IDENTITY,
            inputs: HashMap::new(),
        }
    }

    /// Returns the pose and inputs to their defaults. Emulation stays enabled and the profile
    /// selection is kept, since those are configuration rather than state.
    pub fn reset(&mut self, settings: &ControllerSettings, hand: Hand) {
        self.position = settings.start_positions[hand.index()];
        self.orientation = Quat::IDENTITY;
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
