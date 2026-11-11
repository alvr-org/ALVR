//! Emulated hand tracking: the articulation model, the gesture library, and the settings file.
//!
//! ALVR carries hand tracking as the 26 joint poses of OpenXR's `XR_EXT_hand_tracking`, in world
//! space, and the server turns those into SteamVR's 31-bone skeleton itself
//! (`server_openvr::tracking::to_openvr_ffi_hand_skeleton`). So the emulator's job is to produce a
//! plausible 26-joint skeleton, and everything downstream is the real path.
//!
//! Posing those 26 joints directly would mean 26 free rotations, most combinations of which are
//! not hands. Instead a pose is seven numbers — a curl per finger, one splay for the hand, and the
//! thumb's opposition — and the joint angles follow from them; see [`HandPose`]. That is the design
//! document's parameterisation with one refinement: splay is attenuated as the fingers curl, since
//! a fist cannot fan.
//!
//! Poses are built in the **palm frame**, which is OpenXR's palm joint verbatim: the spec puts it
//! "at the center of the middle finger's metacarpal bone", with "+Z parallel to the middle
//! finger's metacarpal bone, pointing away from the finger tips" and "+Y perpendicular to palm
//! surface and pointing towards the back of the hand". So -Z runs towards the fingertips, +Y out
//! of the back of the hand, and +X completes it — pointing towards the thumb on the left hand and
//! away from it on the right. The finger joints follow the spec's own convention too (-Z along the
//! bone towards the tip, +Y out of the back of the finger nail), so the joints this produces need
//! no conversion anywhere. The middle metacarpal therefore lies exactly on the palm frame's Z
//! axis, which is what the constants below encode; the wrist sits behind it, slightly off that
//! axis, as a real one does.

use crate::controllers::Hand;
use alvr_common::{
    Pose,
    anyhow::{Context, Result},
    glam::{Quat, Vec3},
    info, warn,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    f32::consts::PI,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Name of the hand settings file loaded from the executable's directory.
pub const SETTINGS_FILE_NAME: &str = "hands.json";

/// Joints per hand, as `TrackingData::hand_skeletons` carries them.
pub const JOINT_COUNT: usize = 26;

/// Canonical joint names, in OpenXR's `XrHandJointEXT` order.
///
/// Also the names a hand model's skin joints are matched against, lower-cased and stripped of
/// separators, so a rig calling a bone `Index_Proximal_L` resolves to `index_proximal`.
pub const JOINT_NAMES: [&str; JOINT_COUNT] = [
    "palm",
    "wrist",
    "thumb_metacarpal",
    "thumb_proximal",
    "thumb_distal",
    "thumb_tip",
    "index_metacarpal",
    "index_proximal",
    "index_intermediate",
    "index_distal",
    "index_tip",
    "middle_metacarpal",
    "middle_proximal",
    "middle_intermediate",
    "middle_distal",
    "middle_tip",
    "ring_metacarpal",
    "ring_proximal",
    "ring_intermediate",
    "ring_distal",
    "ring_tip",
    "little_metacarpal",
    "little_proximal",
    "little_intermediate",
    "little_distal",
    "little_tip",
];

pub const PALM: usize = 0;
pub const WRIST: usize = 1;

/// The five digits, in the order the pose arrays use.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Finger {
    Thumb = 0,
    Index = 1,
    Middle = 2,
    Ring = 3,
    Little = 4,
}

impl Finger {
    pub const ALL: [Finger; 5] = [
        Finger::Thumb,
        Finger::Index,
        Finger::Middle,
        Finger::Ring,
        Finger::Little,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// The hand's joint indices for this finger, base to tip. Four for the thumb, five otherwise.
    pub fn joints(self) -> &'static [usize] {
        const THUMB: [usize; 4] = [2, 3, 4, 5];
        const INDEX: [usize; 5] = [6, 7, 8, 9, 10];
        const MIDDLE: [usize; 5] = [11, 12, 13, 14, 15];
        const RING: [usize; 5] = [16, 17, 18, 19, 20];
        const LITTLE: [usize; 5] = [21, 22, 23, 24, 25];

        match self {
            Finger::Thumb => &THUMB,
            Finger::Index => &INDEX,
            Finger::Middle => &MIDDLE,
            Finger::Ring => &RING,
            Finger::Little => &LITTLE,
        }
    }
}

/// How a hand is articulated, independent of where it is.
///
/// Seven numbers, all 0..1, chosen so that every combination is a hand a hand can actually make.
/// A joint-angle representation would be 20-odd values most of whose combinations are anatomically
/// impossible; these map onto the joints through [`skeleton`], which spreads a finger's curl across
/// its knuckles the way the tendons do.
#[derive(Clone, Copy, PartialEq)]
pub struct HandPose {
    /// Per finger, thumb first: 0 fully extended, 1 fully curled.
    pub curl: [f32; 5],
    /// 0 fingers together, 1 fully splayed. Attenuated as the fingers curl, since a fist cannot
    /// fan.
    pub spread: f32,
    /// The thumb's one extra freedom: 0 lies alongside the fingers in the plane of the palm, 1 is
    /// fully opposed, swung across the palm with the pad facing the fingertips. Applies whether
    /// the thumb is curled or not.
    pub thumb_opposition: f32,
}

impl HandPose {
    /// A flat, fully open hand with the fingers together. The reference pose: every angle is zero,
    /// which is also what a hand model's bind pose is aligned against.
    pub const FLAT: Self = Self {
        curl: [0.0; 5],
        spread: 0.0,
        thumb_opposition: 0.0,
    };

    fn clamped(self) -> Self {
        Self {
            curl: self.curl.map(|value| value.clamp(0.0, 1.0)),
            spread: self.spread.clamp(0.0, 1.0),
            thumb_opposition: self.thumb_opposition.clamp(0.0, 1.0),
        }
    }

    fn lerp(self, other: Self, alpha: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * alpha;

        Self {
            curl: std::array::from_fn(|index| mix(self.curl[index], other.curl[index])),
            spread: mix(self.spread, other.spread),
            thumb_opposition: mix(self.thumb_opposition, other.thumb_opposition),
        }
    }
}

/// One selectable articulation, loaded from the settings file.
pub struct NamedPose {
    pub name: String,
    /// Shown as the button's tooltip.
    pub description: String,
    pub pose: HandPose,
    /// Transition time into this pose, when it overrides the global default.
    pub transition: Option<Duration>,
}

/// One step of a gesture: a pose, and where in the sequence the hand should be in it.
pub struct Keyframe {
    /// Index into [`HandSettings::poses`].
    pub pose: usize,
    /// 0 at the start of the gesture, 1 at its end.
    pub phase: f32,
}

/// A timed sequence of poses — what a hand *does* rather than how it is held.
///
/// A pose alone cannot express a tap: holding a pinch and releasing it by hand means getting the
/// timing right by mouse, which is fiddly and unrepeatable. A gesture plays a keyframed path
/// through the poses over a fixed duration, so `Point` at 0, `Pinch` at 0.5, `Point` at 1 over half
/// a second is a click, and clicking its button performs exactly that every time.
pub struct Gesture {
    pub name: String,
    /// Shown as the button's tooltip.
    pub description: String,
    /// Sorted by phase, at least one entry.
    pub keyframes: Vec<Keyframe>,
    pub duration: Duration,
}

impl Gesture {
    /// The articulation this gesture calls for at a point in its run, `phase` from 0 to 1.
    ///
    /// Each segment eases in and out rather than running linearly, so a tap accelerates away from
    /// one pose and settles into the next the way a finger does, instead of moving at a constant
    /// rate and stopping dead.
    pub fn sample(&self, poses: &[NamedPose], phase: f32) -> HandPose {
        let at = |keyframe: &Keyframe| {
            poses
                .get(keyframe.pose)
                .map(|pose| pose.pose)
                .unwrap_or(HandPose::FLAT)
        };

        let Some(first) = self.keyframes.first() else {
            return HandPose::FLAT;
        };

        if phase <= first.phase {
            return at(first);
        }

        for pair in self.keyframes.windows(2) {
            if phase <= pair[1].phase {
                let span = pair[1].phase - pair[0].phase;
                let alpha = if span > f32::EPSILON {
                    (phase - pair[0].phase) / span
                } else {
                    1.0
                };

                return at(&pair[0]).lerp(at(&pair[1]), alpha * alpha * (3.0 - 2.0 * alpha));
            }
        }

        at(self.keyframes.last().expect("checked non-empty"))
    }

    /// The pose the hand is left holding once the gesture finishes.
    pub fn final_pose(&self) -> Option<usize> {
        self.keyframes.last().map(|keyframe| keyframe.pose)
    }

    /// The articulation that best stands for this gesture in an icon: the keyframe furthest from
    /// the one it starts in, which is the part of the movement worth showing. A click drawn as its
    /// starting point would be indistinguishable from the pose it starts in.
    pub fn signature_pose(&self, poses: &[NamedPose]) -> HandPose {
        let at = |keyframe: &Keyframe| {
            poses
                .get(keyframe.pose)
                .map(|pose| pose.pose)
                .unwrap_or(HandPose::FLAT)
        };

        let Some(first) = self.keyframes.first().map(at) else {
            return HandPose::FLAT;
        };

        let distance = |pose: &HandPose| {
            pose.curl
                .iter()
                .zip(first.curl)
                .map(|(a, b)| (a - b).abs())
                .sum::<f32>()
                + (pose.spread - first.spread).abs()
                + (pose.thumb_opposition - first.thumb_opposition).abs()
        };

        self.keyframes
            .iter()
            .map(at)
            .max_by(|a, b| distance(a).total_cmp(&distance(b)))
            .unwrap_or(first)
    }
}

// Anatomy of the modelled hand, in metres, for a nominal hand 190 mm from wrist to middle
// fingertip. `hand_length` in the settings scales all of it. Values are approximate adult
// measurements; what matters is that the proportions are right, since the gesture recognition on
// the server measures real distances between fingertips.

/// Wrist to middle fingertip of the built-in proportions, which `hand_length` scales against.
const NOMINAL_HAND_LENGTH: f32 = 0.190;

/// Wrist joint.
const WRIST_POSITION: Vec3 = Vec3::new(-0.0015, 0.0027, 0.0605);

/// Metacarpal bases (the carpometacarpal joints), where each finger's chain starts. Close
/// together at the wrist; the knuckles below fan much wider, which is what gives the palm its
/// shape. The middle finger's sits on the palm frame's axis, since the palm joint is defined as
/// the centre of that bone.
const METACARPAL_BASE: [Vec3; 4] = [
    Vec3::new(0.0070, -0.0028, 0.0239),
    Vec3::new(0.0000, 0.0000, 0.0328),
    Vec3::new(-0.0106, -0.0013, 0.0334),
    Vec3::new(-0.0257, -0.0032, 0.0264),
];

/// Knuckles (the metacarpophalangeal joints) at rest, slightly arched so the index and middle sit
/// further forward than the little.
const KNUCKLE: [Vec3; 4] = [
    Vec3::new(0.022, 0.000, -0.033),
    Vec3::new(0.000, 0.000, -0.0328),
    Vec3::new(-0.021, 0.000, -0.030),
    Vec3::new(-0.041, 0.000, -0.024),
];

/// Proximal, intermediate and distal phalanx lengths of the four fingers. The distal one runs to
/// the fingertip rather than to the end of the bone, since that is what the gesture recognition on
/// the server measures against.
const PHALANX: [[f32; 3]; 4] = [
    [0.040, 0.024, 0.023],
    [0.045, 0.028, 0.024],
    [0.042, 0.027, 0.023],
    [0.033, 0.019, 0.020],
];

/// Where the thumb's carpometacarpal joint sits: lateral, towards the palm, and proximal.
const THUMB_BASE: Vec3 = Vec3::new(0.015, -0.012, 0.022);
/// Thumb metacarpal, proximal phalanx and distal phalanx.
const THUMB_BONES: [f32; 3] = [0.043, 0.032, 0.028];

/// Flexion at full curl, in degrees: knuckle, middle joint, fingertip joint.
const FLEXION: [f32; 3] = [85.0, 100.0, 68.0];
/// The same for the thumb, whose knuckle and single interphalangeal joint bend far less. In a fist
/// the thumb lies across the curled fingers with its last joint only partly bent; bending it as far
/// as a finger's hooks the tip back towards the wrist.
const THUMB_FLEXION: [f32; 2] = [55.0, 45.0];
/// Curling the ring and little fingers cups the palm by rotating their metacarpals, which a real
/// hand does at the carpometacarpal joints and a flat palm cannot fake.
const CUP: [f32; 4] = [0.0, 0.0, 8.0, 15.0];

/// Knuckle abduction with the fingers together and fully splayed, in degrees. Positive fans
/// towards the thumb.
const ABDUCTION_TOGETHER: [f32; 4] = [1.0, 0.0, -1.5, -4.0];
const ABDUCTION_APART: [f32; 4] = [16.0, 3.0, -9.0, -22.0];

// The thumb's base frame is given by two directions rather than by a chain of angles, because
// opposition is a rotation about no axis the palm has: the metacarpal swings across in front of
// the palm *and* the whole digit rolls, so that flexing it afterwards carries the tip towards the
// fingers instead of towards the palm. Angles about the palm's axes cannot express that without
// interacting; two interpolated directions can.
//
// `THUMB_ALONG` is the direction of the metacarpal, and `THUMB_CURL_TOWARDS` is where flexing the
// thumb takes its tip. Each pair is the value at opposition 0 and at opposition 1.

/// Metacarpal direction: beside the index finger in the plane of the palm, then swung down in
/// front of the palm. Opposed, it points forward and into the palm, barely outwards: swinging it
/// out to the side too splays the curled thumb away from the fist.
const THUMB_ALONG: [Vec3; 2] = [
    Vec3::new(0.50, -0.10, -0.86),
    Vec3::new(0.155, -0.65, -0.745),
];

/// Where flexion takes the thumb: across the palm towards the little finger, whether the thumb is
/// opposed or not. A thumb's flexion runs parallel to the palm rather than into it, unlike a
/// finger's; curling it straight into the palm carries it out of the hand instead of round the
/// fist. The values are derived from where a real thumb's joints sit in a fist, against this
/// hand's own knuckles.
const THUMB_CURL_TOWARDS: [Vec3; 2] =
    [Vec3::new(-0.90, -0.40, 0.0), Vec3::new(-0.85, -0.45, -0.25)];

fn degrees(value: f32) -> f32 {
    value * PI / 180.0
}

fn mix(range: [f32; 2], alpha: f32) -> f32 {
    range[0] + (range[1] - range[0]) * alpha
}

/// Builds the 26 joint poses of one articulated hand, in the palm frame.
///
/// The palm joint is the identity, so composing every joint with the palm's world pose is all that
/// is needed to place the hand — which is exactly what the emulator's 6DoF pose controls.
///
/// `length` is the wrist-to-fingertip size of the hand, which scales the whole skeleton.
pub fn skeleton(pose: &HandPose, hand: Hand, length: f32) -> [Pose; JOINT_COUNT] {
    let pose = pose.clamped();

    // The right hand is the left hand mirrored across the palm's YZ plane. Mirroring positions is
    // a sign on X; mirroring rotations is a sign on the angles about Y and Z, which keeps them
    // proper rotations rather than reflections.
    let side = match hand {
        Hand::Left => 1.0,
        Hand::Right => -1.0,
    };
    let scale = length / NOMINAL_HAND_LENGTH;

    let mirrored = |position: Vec3| Vec3::new(position.x * side, position.y, position.z) * scale;
    let fan = |angle: f32| Quat::from_rotation_y(-side * angle);
    let flex = |angle: f32| Quat::from_rotation_x(-angle);

    let mut joints = [Pose::IDENTITY; JOINT_COUNT];

    // Deliberately not the spec's frame (+Z towards the middle metacarpal): with that joint exactly
    // on the wrist's axis, SteamVR snaps the wrist by up to 180° from frame to frame.
    joints[WRIST] = Pose {
        orientation: Quat::IDENTITY,
        position: mirrored(WRIST_POSITION),
    };

    for finger in [Finger::Index, Finger::Middle, Finger::Ring, Finger::Little] {
        let slot = finger.index() - 1;
        let chain = finger.joints();
        let curl = pose.curl[finger.index()];

        // The metacarpal's own direction comes from the palm's shape: base to knuckle. Curling the
        // outer fingers additionally cups the palm.
        let base = mirrored(METACARPAL_BASE[slot]);
        let knuckle = mirrored(KNUCKLE[slot]);
        let metacarpal_length = (knuckle - base).length();

        let metacarpal = look_along(knuckle - base) * flex(degrees(CUP[slot]) * curl);

        joints[chain[0]] = Pose {
            orientation: metacarpal,
            position: base,
        };

        // Splay opens at the knuckle, and closes again as the finger curls: the fingers of a fist
        // are gathered whatever the splay control says.
        let abduction = degrees(mix(
            [ABDUCTION_TOGETHER[slot], ABDUCTION_APART[slot]],
            pose.spread,
        )) * (1.0 - 0.7 * curl);

        let mut orientation = metacarpal * fan(abduction) * flex(degrees(FLEXION[0]) * curl);
        let mut position = joints[chain[0]].position + metacarpal * forward(metacarpal_length);

        for (bone, length) in PHALANX[slot].iter().enumerate() {
            joints[chain[bone + 1]] = Pose {
                orientation,
                position,
            };

            position += orientation * forward(*length * scale);
            // The last bone ends at the fingertip, which keeps its parent's orientation.
            if let Some(flexion) = FLEXION.get(bone + 1) {
                orientation *= flex(degrees(*flexion) * curl);
            }
        }

        joints[chain[4]] = Pose {
            orientation,
            position,
        };
    }

    // The thumb: one extra freedom at the base, and one joint fewer than the fingers.
    let opposition = pose.thumb_opposition;
    let curl = pose.curl[Finger::Thumb.index()];

    let direction = |pair: [Vec3; 2]| {
        let blended = pair[0].lerp(pair[1], opposition);

        Vec3::new(blended.x * side, blended.y, blended.z)
    };

    let mut orientation = frame_facing(direction(THUMB_ALONG), direction(THUMB_CURL_TOWARDS));
    let mut position = mirrored(THUMB_BASE);

    let chain = Finger::Thumb.joints();
    for (bone, length) in THUMB_BONES.iter().enumerate() {
        joints[chain[bone]] = Pose {
            orientation,
            position,
        };

        position += orientation * forward(*length * scale);
        if let Some(flexion) = THUMB_FLEXION.get(bone) {
            orientation *= flex(degrees(*flexion) * curl);
        }
    }

    joints[chain[3]] = Pose {
        orientation,
        position,
    };

    joints
}

/// A vector of the given length along a joint's bone direction, which is its local -Z.
fn forward(length: f32) -> Vec3 {
    Vec3::new(0.0, 0.0, -length)
}

/// The direction a hand points, in the palm frame: along the index metacarpal, from its base to the
/// knuckle, which is the line the extended index finger continues.
///
/// Constant rather than read from the live pose: an axis that swung about as the fingers curled
/// would make the hand impossible to aim with, and the metacarpal does not articulate. The driver
/// points SteamVR's `/pose/tip` along the same bone (`server_openvr::tracking::hand_tip_offset`),
/// so the Aim gizmo and the application's ray agree.
pub fn aim_direction(hand: Hand) -> Vec3 {
    let flat = skeleton(&HandPose::FLAT, hand, NOMINAL_HAND_LENGTH);
    let chain = Finger::Index.joints();

    (flat[chain[1]].position - flat[chain[0]].position).normalize_or(Vec3::NEG_Z)
}

/// Where a held tube's axis runs through the hand, in the palm frame of `joints`: the centre of the
/// loop the middle finger closes in a fist. OpenXR puts the grip pose here, "at the centroid of the
/// user's palm when the user makes a fist or holds a tube-like object".
///
/// Constant for the same reason as [`aim_direction`] — a grip that moved as the fingers curled
/// would shake whatever the hand holds — and scaled to `joints` by the middle metacarpal, the one
/// bone no pose can change.
pub fn fist_centre(joints: &[Pose; JOINT_COUNT]) -> Vec3 {
    const FIST: HandPose = HandPose {
        curl: [1.0; 5],
        spread: 0.0,
        thumb_opposition: 0.0,
    };

    // Mirroring only negates X, which the centre does not use, so either hand will do.
    let fist = skeleton(&FIST, Hand::Left, NOMINAL_HAND_LENGTH);
    let chain = Finger::Middle.joints();

    // The loop in the palm's midplane, as (Z, Y): the palm, then out along the finger and round to
    // its curled tip, closing back at the palm. Its area centroid, by the shoelace formula, rather
    // than the mean of its corners, which crowd together at the curled end.
    let corners: Vec<(f32, f32)> = std::iter::once(PALM)
        .chain(chain[1..].iter().copied())
        .map(|joint| (fist[joint].position.z, fist[joint].position.y))
        .collect();

    let (mut area, mut z, mut y) = (0.0, 0.0, 0.0);
    for (index, &(z0, y0)) in corners.iter().enumerate() {
        let (z1, y1) = corners[(index + 1) % corners.len()];
        let cross = z0 * y1 - z1 * y0;

        area += cross;
        z += (z0 + z1) * cross;
        y += (y0 + y1) * cross;
    }
    let centre = Vec3::new(0.0, y / (3.0 * area), z / (3.0 * area)) - fist[PALM].position;

    let metacarpal =
        |joints: &[Pose; JOINT_COUNT]| joints[PALM].position.distance(joints[chain[1]].position);

    joints[PALM].position + centre * (metacarpal(joints) / metacarpal(&fist))
}

/// Whether a joint lies beyond the second knuckle — the part of a finger that a fingerless glove
/// leaves bare, which is where the model this ships against has its cuff ridge. Used only to
/// two-tone such a model; see `glove_color` in the settings.
pub fn beyond_glove_cuff(joint: usize) -> bool {
    Finger::ALL
        .into_iter()
        .any(|finger| finger.joints().iter().skip(2).any(|index| *index == joint))
}

/// The joint whose position gives another joint's bone direction. Fingertips have no child and
/// borrow their parent's frame; the palm and the wrist point at the middle finger's knuckle, which
/// is the closest thing a hand has to a forward direction.
fn bone_child(joint: usize) -> Option<usize> {
    const MIDDLE_KNUCKLE: usize = 12;

    if joint == PALM || joint == WRIST {
        return Some(MIDDLE_KNUCKLE);
    }

    Finger::ALL.into_iter().find_map(|finger| {
        let chain = finger.joints();
        let position = chain.iter().position(|index| *index == joint)?;

        chain.get(position + 1).copied()
    })
}

/// Orientations derived purely from where the joints are: bone along -Z, back of the hand along
/// +Y. Joints whose position — or whose child's — is unknown come back as `None`.
///
/// This is what lets a hand model of any rig convention be driven by these joints. Both the
/// model's bind pose and this skeleton are reduced to the same geometric frame, and the constant
/// difference between a rig's own bone frame and that one is the correction applied when posing
/// it; see [`crate::skinned`].
pub fn canonical_frames(
    positions: &[Option<Vec3>; JOINT_COUNT],
    hand: Hand,
) -> [Option<Quat>; JOINT_COUNT] {
    let side = match hand {
        Hand::Left => 1.0,
        Hand::Right => -1.0,
    };

    // The back of the hand, from the triangle the wrist and the outer knuckles make. Its sign
    // depends on which hand this is, since the two are mirror images.
    let dorsal = (|| {
        let wrist = positions[WRIST]?;
        let index = positions[Finger::Index.joints()[1]]?;
        let little = positions[Finger::Little.joints()[1]]?;

        Some((index - wrist).cross(little - wrist).normalize_or_zero() * side)
    })();

    let Some(dorsal) = dorsal.filter(|dorsal| *dorsal != Vec3::ZERO) else {
        return [None; JOINT_COUNT];
    };

    let mut frames = [None; JOINT_COUNT];

    for joint in 0..JOINT_COUNT {
        let Some(position) = positions[joint] else {
            continue;
        };

        let along = bone_child(joint)
            .and_then(|child| positions[child])
            .map(|child| child - position);

        frames[joint] = along.map(|along| frame_facing(along, -dorsal));
    }

    // A fingertip has no bone of its own, so it keeps the frame of the joint before it.
    for finger in Finger::ALL {
        let chain = finger.joints();
        let tip = chain[chain.len() - 1];
        let parent = chain[chain.len() - 2];

        if frames[tip].is_none() {
            frames[tip] = frames[parent];
        }
    }

    frames
}

/// The constant rotation between each joint's declared orientation in [`skeleton`] and the
/// geometric frame [`canonical_frames`] derives from the same flat hand.
///
/// Posing a model means composing this with the model's own equivalent, so that a rig's bone
/// convention and this one are both reduced to the same reference before they are compared.
pub fn frame_corrections(hand: Hand, length: f32) -> [Quat; JOINT_COUNT] {
    let flat = skeleton(&HandPose::FLAT, hand, length);
    let positions = std::array::from_fn(|joint| Some(flat[joint].position));
    let frames = canonical_frames(&positions, hand);

    std::array::from_fn(|joint| match frames[joint] {
        Some(frame) => flat[joint].orientation.conjugate() * frame,
        None => Quat::IDENTITY,
    })
}

/// The rotation taking the palm frame's -Z onto `direction`, without rolling about it.
fn look_along(direction: Vec3) -> Quat {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return Quat::IDENTITY;
    }

    Quat::from_rotation_arc(Vec3::NEG_Z, direction)
}

/// A joint frame whose bone runs `along` and whose flexion carries the tip towards `curl_towards`.
///
/// In the joint convention used throughout, the bone is the frame's -Z and flexion (a negative
/// rotation about +X) moves the tip towards -Y, so this is just an orthonormal basis built from
/// those two directions. Deriving X as `Y × Z` also mirrors correctly: mirroring the two input
/// directions produces the mirrored *rotation*, not a reflection, which is what keeps the right
/// hand a proper rotation of the left.
fn frame_facing(along: Vec3, curl_towards: Vec3) -> Quat {
    let z_axis = -along.normalize_or_zero();
    if z_axis == Vec3::ZERO {
        return Quat::IDENTITY;
    }

    // Only the component of the curl direction perpendicular to the bone means anything; the
    // parallel part would just shorten the finger.
    let y_axis = (-curl_towards - z_axis * -curl_towards.dot(z_axis)).normalize_or_zero();
    if y_axis == Vec3::ZERO {
        return look_along(along);
    }

    Quat::from_mat3(&alvr_common::glam::Mat3::from_cols(
        y_axis.cross(z_axis),
        y_axis,
        z_axis,
    ))
}

/// Live state of one emulated hand. Owned by the UI thread; the HTTP API mutates it through queued
/// commands, so user input and API input merge in one place, as the controllers do.
pub struct HandState {
    pub enabled: bool,
    /// Show the 3D model in the scene view.
    pub model_visible: bool,
    /// Head-relative palm position. Same axes as the controllers: X right, Y up, -Z forward.
    pub position: Vec3,
    /// Head-relative palm orientation.
    pub orientation: Quat,
    /// The selected pose, or `None` when the articulation was set directly through the API and so
    /// belongs to no entry in the pose list.
    pub pose_index: Option<usize>,
    /// The articulation actually being sent, which moves towards the target over the transition
    /// time rather than snapping to it.
    pub pose: HandPose,
    /// The articulation being moved towards.
    target: HandPose,
    /// The running transition: where it started from, when, and how long it takes.
    transition: Option<(HandPose, Instant, Duration)>,
    /// The gesture being played and when it started, if any. A gesture drives the articulation
    /// outright while it runs, so it and a transition are never both live.
    playing: Option<(usize, Instant)>,
}

impl HandState {
    pub fn new(settings: &HandSettings, hand: Hand) -> Self {
        Self {
            enabled: false,
            model_visible: false,
            position: settings.start_positions[hand.index()],
            orientation: settings.start_orientations[hand.index()],
            pose_index: Some(0),
            pose: settings.pose_at(0),
            target: settings.pose_at(0),
            transition: None,
            playing: None,
        }
    }

    /// Returns the pose and the palm to their defaults, and stops any gesture. Emulation stays
    /// enabled, as resetting a controller keeps it enabled.
    pub fn reset(&mut self, settings: &HandSettings, hand: Hand) {
        self.position = settings.start_positions[hand.index()];
        self.orientation = settings.start_orientations[hand.index()];
        self.pose_index = Some(0);
        self.pose = settings.pose_at(0);
        self.target = self.pose;
        self.transition = None;
        self.playing = None;
    }

    /// Starts an animated change to a pose. Selecting the pose already being moved to is a no-op,
    /// so repeated clicks do not restart the animation. `transition` overrides the configured
    /// time, which is what lets the API pose a hand instantly for a scripted test.
    pub fn select_pose(
        &mut self,
        settings: &HandSettings,
        pose: usize,
        transition: Option<Duration>,
    ) {
        let Some(entry) = settings.poses.get(pose) else {
            return;
        };

        if self.pose_index == Some(pose) && self.playing.is_none() {
            return;
        }

        let duration = transition
            .or(entry.transition)
            .unwrap_or(settings.transition);

        self.playing = None;
        self.pose_index = Some(pose);
        self.animate_to(entry.pose, duration);
    }

    /// Moves to an articulation that is not one of the named poses, as the API can ask for. The
    /// hand stops belonging to any pose until one is selected again.
    pub fn set_articulation(&mut self, pose: HandPose, duration: Duration) {
        self.playing = None;
        self.pose_index = None;
        self.animate_to(pose, duration);
    }

    /// Starts a gesture from the beginning. Playing one already running restarts it, which is what
    /// makes a click button repeatable without waiting for the previous run to finish.
    pub fn play_gesture(&mut self, settings: &HandSettings, gesture: usize) {
        if settings.gestures.get(gesture).is_none() {
            return;
        }

        self.transition = None;
        self.playing = Some((gesture, Instant::now()));
    }

    /// The gesture currently playing, if any.
    pub fn playing_gesture(&self) -> Option<usize> {
        self.playing.map(|(gesture, _)| gesture)
    }

    fn animate_to(&mut self, pose: HandPose, duration: Duration) {
        self.target = pose;

        if duration.is_zero() {
            self.pose = pose;
            self.transition = None;
        } else {
            self.transition = Some((self.pose, Instant::now(), duration));
        }
    }

    /// Advances a running gesture or transition. Call once per frame.
    pub fn advance(&mut self, settings: &HandSettings, now: Instant) {
        if let Some((index, started)) = self.playing {
            let Some(gesture) = settings.gestures.get(index) else {
                self.playing = None;
                return;
            };

            let elapsed = now.saturating_duration_since(started).as_secs_f32();
            let phase = if gesture.duration.is_zero() {
                1.0
            } else {
                (elapsed / gesture.duration.as_secs_f32()).clamp(0.0, 1.0)
            };

            self.pose = gesture.sample(&settings.poses, phase);

            if phase >= 1.0 {
                // The hand is left holding whatever the gesture ended on, and the pose panel shows
                // it selected — a click that ends on Point leaves the hand pointing.
                self.playing = None;
                self.pose_index = gesture.final_pose();
                self.target = self.pose;
            }

            return;
        }

        let Some((from, started, duration)) = self.transition else {
            return;
        };

        let elapsed = now.saturating_duration_since(started).as_secs_f32();
        let alpha = if duration.is_zero() {
            1.0
        } else {
            (elapsed / duration.as_secs_f32()).clamp(0.0, 1.0)
        };

        // Smoothstep, so the hand eases out of one pose and into the next rather than starting and
        // stopping abruptly, which reads as a snap even over half a second.
        self.pose = from.lerp(self.target, alpha * alpha * (3.0 - 2.0 * alpha));

        if alpha >= 1.0 {
            self.pose = self.target;
            self.transition = None;
        }
    }

    /// How far a running pose change or gesture has got, for the UI to show progress. `None` when
    /// the hand has settled.
    pub fn progress(&self, settings: &HandSettings, now: Instant) -> Option<f32> {
        let (started, duration) = match self.playing {
            Some((index, started)) => (started, settings.gestures.get(index)?.duration),
            None => {
                let (_, started, duration) = self.transition?;

                (started, duration)
            }
        };

        if duration.is_zero() {
            return None;
        }

        Some(
            (now.saturating_duration_since(started).as_secs_f32() / duration.as_secs_f32())
                .clamp(0.0, 1.0),
        )
    }

    /// The hand's joints in the palm frame, which is what both the renderer and the wire need
    /// before the palm pose is applied.
    pub fn local_skeleton(&self, settings: &HandSettings, hand: Hand) -> [Pose; JOINT_COUNT] {
        skeleton(&self.pose, hand, settings.hand_length)
    }
}

/// Hand emulation settings, resolved from the settings file.
pub struct HandSettings {
    /// Radians of palm rotation per pixel of drag on the rotation pads, as the controllers have.
    pub rotation_sensitivity: f32,
    /// Head-relative starting palm position per hand. Left is index 0.
    pub start_positions: [Vec3; 2],
    /// Head-relative starting palm orientation per hand, from the configured pitch and roll.
    pub start_orientations: [Quat; 2],
    /// Wrist to middle fingertip, which scales the whole skeleton.
    pub hand_length: f32,
    /// Default time a change of pose takes.
    pub transition: Duration,
    pub poses: Vec<NamedPose>,
    pub gestures: Vec<Gesture>,
    /// Optional glTF hand models, one per hand. Left is index 0.
    pub models: [Option<PathBuf>; 2],
    /// Base colour the hand model is drawn in, since a hand model rarely carries a material.
    pub model_color: [f32; 3],
    /// Colour for the part of the model a fingerless glove covers, when it is one. `None` draws
    /// the whole model in `model_color`.
    pub glove_color: Option<[f32; 3]>,
    /// Overrides mapping a model's own joint node names onto the canonical names in
    /// [`JOINT_NAMES`], for a rig whose names do not match.
    pub joint_names: HashMap<String, String>,
}

impl HandSettings {
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
                        warn!("Cannot write default hand settings: {e}");
                    } else {
                        info!("Wrote default hand settings to {}", path.display());
                    }
                }
                Err(e) => warn!("Cannot serialise default hand settings: {e}"),
            }
        }

        let file = match load_file(&path) {
            Ok(file) => file,
            Err(e) => {
                // Moved aside rather than left in place: a file this cannot read is a file the
                // user can no longer change anything with, and one written by an older layout of
                // these settings is exactly the case that hits it.
                let stale = path.with_extension("json.old");

                warn!(
                    "Cannot load {}: {e:#}. Moving it to {} and writing fresh defaults.",
                    path.display(),
                    stale.display()
                );

                if std::fs::rename(&path, &stale).is_ok()
                    && let Ok(json) = serde_json::to_string_pretty(&default_file())
                {
                    std::fs::write(
                        &path,
                        json + "
",
                    )
                    .ok();
                }

                default_file()
            }
        };

        let mut settings = resolve(file, models_dir);

        // The pose list indexes state, so it must never be empty.
        if settings.poses.is_empty() {
            warn!("Hand settings contain no poses; using the built-in ones");

            let built_in = resolve(default_file(), models_dir);
            settings.poses = built_in.poses;
            settings.gestures = built_in.gestures;
        }

        settings
    }

    /// The articulation of a named pose, or a flat hand if the index is stale.
    pub fn pose_at(&self, index: usize) -> HandPose {
        self.poses
            .get(index)
            .map(|pose| pose.pose)
            .unwrap_or(HandPose::FLAT)
    }

    /// Finds a pose by name, case-insensitively.
    pub fn find_pose(&self, name: &str) -> Option<usize> {
        self.poses
            .iter()
            .position(|pose| pose.name.eq_ignore_ascii_case(name))
    }

    /// Finds a gesture by name, case-insensitively.
    pub fn find_gesture(&self, name: &str) -> Option<usize> {
        self.gestures
            .iter()
            .position(|gesture| gesture.name.eq_ignore_ascii_case(name))
    }
}

/// On-disk form of [`HandSettings`]. Kept separate so the file format stays plain data.
#[derive(Serialize, Deserialize)]
struct SettingsFile {
    rotation_sensitivity: f32,
    left_start_position: [f32; 3],
    right_start_position: [f32; 3],
    /// Upward pitch of the resting palm, in degrees.
    start_pitch_degrees: f32,
    /// Inward roll of the resting palm, in degrees, so the two hands face each other a little the
    /// way they hang.
    start_roll_degrees: f32,
    /// Wrist to middle fingertip, in metres.
    hand_length: f32,
    /// Seconds a change of pose takes, unless the pose overrides it.
    transition_seconds: f32,
    /// Base colour of the hand model, linear RGB.
    model_color: [f32; 3],
    /// Colour of the glove on a fingerless-glove model such as the bundled one, whose cuff ridge
    /// sits at the second knuckle. Everything from there to the fingertips takes `model_color` instead, blended across the weights so the seam is clean.
    /// Set to `null` for a plain hand model, which draws entirely in `model_color`.
    ///
    /// Defaulted rather than optional so a settings file written before this existed picks the
    /// glove up on the next run instead of silently keeping the old single-colour look.
    #[serde(default = "default_glove_color")]
    glove_color: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    left_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    right_model: Option<String>,
    /// Maps a model's own joint node names onto the canonical joint names, for rigs whose names
    /// do not match. Names are compared with case and separators ignored, so this is only needed
    /// for genuinely different naming.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    joint_names: HashMap<String, String>,
    /// The articulations the hand can be put into, shown as the outer grid of the panel.
    #[serde(default)]
    poses: Vec<PoseEntry>,
    /// Timed sequences of those poses, shown as the inner grid.
    #[serde(default)]
    gestures: Vec<GestureEntry>,
}

#[derive(Serialize, Deserialize)]
struct PoseEntry {
    name: String,
    description: String,
    /// Curl of each digit: 0 extended, 1 fully curled.
    #[serde(default)]
    thumb: f32,
    #[serde(default)]
    index: f32,
    #[serde(default)]
    middle: f32,
    #[serde(default)]
    ring: f32,
    #[serde(default)]
    little: f32,
    /// 0 fingers together, 1 fully splayed.
    #[serde(default)]
    spread: f32,
    /// 0 thumb alongside the fingers, 1 fully opposed across the palm.
    #[serde(default)]
    thumb_opposition: f32,
    /// Overrides the global transition time when moving into this pose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    transition_seconds: Option<f32>,
}

/// A gesture as the file describes it: named poses at points along a timeline.
#[derive(Serialize, Deserialize)]
struct GestureEntry {
    name: String,
    description: String,
    /// How long the whole sequence takes.
    duration_seconds: f32,
    keyframes: Vec<KeyframeEntry>,
}

#[derive(Serialize, Deserialize)]
struct KeyframeEntry {
    /// The name of one of the poses above.
    pose: String,
    /// 0 at the start of the gesture, 1 at its end.
    phase: f32,
}

/// The glove tint a settings file gets when it does not mention one, which is the colour that
/// suits the bundled model.
fn default_glove_color() -> Option<[f32; 3]> {
    Some([0.20, 0.22, 0.28])
}

fn load_file(path: &Path) -> Result<SettingsFile> {
    let text = std::fs::read_to_string(path).context("Cannot read hand settings")?;
    serde_json::from_str(&text).context("Cannot parse hand settings")
}

/// The default settings file: the four poses the design document calls for, a flat hand, and a few
/// gestures built from them. More of either can be added by editing the file; nothing in the code
/// enumerates them.
fn default_file() -> SettingsFile {
    let poses = vec![
        PoseEntry {
            name: "Idle".into(),
            description: "Relaxed open hand, the pose a hand falls into on its own".into(),
            thumb: 0.15,
            index: 0.14,
            middle: 0.16,
            ring: 0.2,
            little: 0.24,
            spread: 0.35,
            thumb_opposition: 0.3,
            transition_seconds: None,
        },
        PoseEntry {
            name: "Grasp".into(),
            description: "All fingers curled into a fist, thumb across the front".into(),
            thumb: 1.0,
            index: 1.0,
            middle: 1.0,
            ring: 1.0,
            little: 1.0,
            spread: 0.0,
            thumb_opposition: 0.55,
            transition_seconds: None,
        },
        PoseEntry {
            name: "Point".into(),
            description: "Index finger extended, the other fingers curled into a fist".into(),
            thumb: 0.85,
            index: 0.0,
            middle: 1.0,
            ring: 1.0,
            little: 1.0,
            spread: 0.0,
            thumb_opposition: 0.5,
            transition_seconds: None,
        },
        PoseEntry {
            name: "Pinch".into(),
            description: "Thumb and index fingertips touching, the rest curled".into(),
            thumb: 0.15,
            index: 0.45,
            middle: 0.95,
            ring: 1.0,
            little: 1.0,
            spread: 0.0,
            thumb_opposition: 0.7,
            transition_seconds: None,
        },
        PoseEntry {
            name: "Plank".into(),
            description: "Flat open hand, every finger and the thumb straight".into(),
            thumb: 0.0,
            index: 0.0,
            middle: 0.0,
            ring: 0.0,
            little: 0.0,
            spread: 0.0,
            thumb_opposition: 0.0,
            transition_seconds: None,
        },
    ];

    SettingsFile {
        rotation_sensitivity: 0.005,
        left_start_position: [-0.18, -0.25, -0.35],
        right_start_position: [0.18, -0.25, -0.35],
        start_pitch_degrees: 20.0,
        start_roll_degrees: 15.0,
        hand_length: 0.185,
        transition_seconds: 0.5,
        model_color: [0.78, 0.62, 0.52],
        glove_color: default_glove_color(),
        left_model: Some("models/hand_left.gltf".into()),
        right_model: Some("models/hand_right.gltf".into()),
        joint_names: HashMap::new(),
        poses,
        gestures: default_gestures(),
    }
}

/// The built-in gestures.
///
/// Only the momentary ones: a tap is impossible to perform reliably by holding a pose with the
/// mouse, because the timing is the whole point. Anything a gesture would only *arrive* at — a
/// closed fist, a held pinch — is already a pose, and duplicating it here would put the same thing
/// in both grids.
fn default_gestures() -> Vec<GestureEntry> {
    let keyframe = |pose: &str, phase: f32| KeyframeEntry {
        pose: pose.into(),
        phase,
    };

    vec![
        GestureEntry {
            name: "Click".into(),
            description: "Pinch and release while pointing, which is a trigger click".into(),
            duration_seconds: 0.5,
            keyframes: vec![
                keyframe("Point", 0.0),
                keyframe("Pinch", 0.5),
                keyframe("Point", 1.0),
            ],
        },
        GestureEntry {
            name: "Double".into(),
            description: "Two clicks in quick succession".into(),
            duration_seconds: 0.8,
            keyframes: vec![
                keyframe("Point", 0.0),
                keyframe("Pinch", 0.25),
                keyframe("Point", 0.5),
                keyframe("Pinch", 0.75),
                keyframe("Point", 1.0),
            ],
        },
    ]
}

/// Settings as they load with no `hands.json` present, for tests elsewhere in the crate that need
/// a hand to pose without reaching into this module's internals.
#[cfg(test)]
pub(crate) fn default_settings() -> HandSettings {
    resolve(default_file(), Path::new("."))
}

fn resolve(file: SettingsFile, directory: &Path) -> HandSettings {
    let poses: Vec<NamedPose> = file
        .poses
        .into_iter()
        .map(|entry| NamedPose {
            name: entry.name,
            description: entry.description,
            pose: HandPose {
                curl: [
                    entry.thumb,
                    entry.index,
                    entry.middle,
                    entry.ring,
                    entry.little,
                ],
                spread: entry.spread,
                thumb_opposition: entry.thumb_opposition,
            },
            transition: entry
                .transition_seconds
                .map(|seconds| Duration::from_secs_f32(seconds.clamp(0.0, 10.0))),
        })
        .collect();

    let find = |name: &str| {
        poses
            .iter()
            .position(|pose| pose.name.eq_ignore_ascii_case(name))
    };

    // A gesture naming a pose that does not exist is dropped rather than silently played with a
    // flat hand, and says which name was wrong.
    let gestures = file
        .gestures
        .into_iter()
        .filter_map(|entry| {
            let mut keyframes = Vec::with_capacity(entry.keyframes.len());

            for keyframe in &entry.keyframes {
                match find(&keyframe.pose) {
                    Some(pose) => keyframes.push(Keyframe {
                        pose,
                        phase: keyframe.phase.clamp(0.0, 1.0),
                    }),
                    None => {
                        warn!(
                            "Gesture '{}' names a pose that does not exist: '{}'",
                            entry.name, keyframe.pose
                        );

                        return None;
                    }
                }
            }

            if keyframes.is_empty() {
                warn!("Gesture '{}' has no keyframes", entry.name);

                return None;
            }

            keyframes.sort_by(|a, b| a.phase.total_cmp(&b.phase));

            Some(Gesture {
                name: entry.name,
                description: entry.description,
                keyframes,
                duration: Duration::from_secs_f32(entry.duration_seconds.clamp(0.0, 60.0)),
            })
        })
        .collect();

    // The palms roll inwards by the same angle in opposite directions, so the pair is symmetric.
    let pitch = Quat::from_rotation_x(degrees(file.start_pitch_degrees));
    let orientation =
        |side: f32| pitch * Quat::from_rotation_z(side * degrees(file.start_roll_degrees));

    HandSettings {
        rotation_sensitivity: file.rotation_sensitivity,
        start_positions: [
            Vec3::from_array(file.left_start_position),
            Vec3::from_array(file.right_start_position),
        ],
        start_orientations: [orientation(-1.0), orientation(1.0)],
        hand_length: file.hand_length.clamp(0.05, 0.5),
        transition: Duration::from_secs_f32(file.transition_seconds.clamp(0.0, 10.0)),
        gestures,
        models: [
            file.left_model.as_ref().map(|model| directory.join(model)),
            file.right_model.as_ref().map(|model| directory.join(model)),
        ],
        model_color: file.model_color,
        glove_color: file.glove_color,
        joint_names: file.joint_names,
        poses,
    }
}

/// Normalises a joint or node name for matching: lower case, separators removed, and any trailing
/// side marker dropped, so `Index_Proximal_L`, `index-proximal` and `IndexProximal` all agree.
pub fn normalize_joint_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let trimmed = lower
        .strip_suffix("_l")
        .or_else(|| lower.strip_suffix("_r"))
        .or_else(|| lower.strip_suffix(".l"))
        .or_else(|| lower.strip_suffix(".r"))
        .unwrap_or(&lower);

    trimmed
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

/// The canonical joint a name refers to, or `None` when it names no joint of a hand.
pub fn joint_from_name(name: &str, overrides: &HashMap<String, String>) -> Option<usize> {
    let name = match overrides.get(name) {
        Some(mapped) => normalize_joint_name(mapped),
        None => normalize_joint_name(name),
    };

    JOINT_NAMES
        .iter()
        .position(|joint| normalize_joint_name(joint) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distances the server's gesture recognition measures, so a gesture that is supposed to read
    /// as a pinch or a fist actually does. Thresholds are ALVR's defaults from
    /// `HandTrackingInteractionConfig`, plus the fingertip radii `hand_gestures.rs` adds.
    const PINCH_CLICK: f32 = 0.014;
    const CURL_TRIGGER: f32 = 0.0365;

    fn settings() -> HandSettings {
        resolve(default_file(), Path::new("."))
    }

    fn joints(pose: &str, hand: Hand) -> [Pose; JOINT_COUNT] {
        let settings = settings();
        let index = settings.find_pose(pose).expect("pose exists");

        skeleton(&settings.pose_at(index), hand, settings.hand_length)
    }

    fn distance(joints: &[Pose; JOINT_COUNT], a: usize, b: usize) -> f32 {
        joints[a].position.distance(joints[b].position)
    }

    /// The hand is the size it says it is, and both hands are mirror images.
    #[test]
    fn proportions_and_mirroring() {
        for gesture in ["Idle", "Grasp", "Point", "Pinch", "Plank"] {
            let left = joints(gesture, Hand::Left);
            let right = joints(gesture, Hand::Right);

            for index in 0..JOINT_COUNT {
                let mirrored = Vec3::new(
                    -right[index].position.x,
                    right[index].position.y,
                    right[index].position.z,
                );

                assert!(
                    left[index].position.distance(mirrored) < 1e-5,
                    "{gesture}: joint {} is not mirrored: {:?} vs {:?}",
                    JOINT_NAMES[index],
                    left[index].position,
                    right[index].position
                );
            }
        }

        let flat = skeleton(&HandPose::FLAT, Hand::Left, 0.185);
        let length = distance(&flat, WRIST, 15);
        assert!(
            (length - 0.185).abs() < 0.006,
            "wrist to middle fingertip is {length} m, expected 0.185"
        );
    }

    /// Every joint of an extended finger lies further from the wrist than the one before it, which
    /// a hand that folds backwards or collapses into itself would fail.
    #[test]
    fn flat_hand_is_flat() {
        let flat = skeleton(&HandPose::FLAT, Hand::Left, 0.185);

        for finger in Finger::ALL {
            let chain = finger.joints();

            for pair in chain.windows(2) {
                let previous = distance(&flat, WRIST, pair[0]);
                let current = distance(&flat, WRIST, pair[1]);

                assert!(
                    current > previous,
                    "{}: {} is not beyond {}",
                    JOINT_NAMES[finger.joints()[0]],
                    JOINT_NAMES[pair[1]],
                    JOINT_NAMES[pair[0]]
                );
            }

            // A flat hand is flat: no finger strays far from the plane of the palm. The thumb is
            // allowed more, because it genuinely sits towards the palm even when extended.
            let allowance = if finger == Finger::Thumb { 0.045 } else { 0.02 };

            for joint in chain {
                let offset = flat[*joint].position.y.abs();
                assert!(
                    offset < allowance,
                    "{}: {} is {offset} m out of the palm plane",
                    JOINT_NAMES[finger.joints()[0]],
                    JOINT_NAMES[*joint]
                );
            }
        }
    }

    /// Pinch has to actually pinch: the server reads a click when the fingertip centres are within
    /// roughly 14 mm, which is the two tips' radii plus its trigger distance.
    #[test]
    fn pinch_touches() {
        let pinch = joints("Pinch", Hand::Left);
        let gap = distance(&pinch, 5, 10);

        assert!(
            gap < PINCH_CLICK,
            "pinch leaves {gap} m between the thumb and index tips, needs under {PINCH_CLICK}"
        );

        // The other gestures must not read as a pinch by accident.
        for gesture in ["Idle", "Point", "Plank"] {
            let joints = joints(gesture, Hand::Left);
            let gap = distance(&joints, 5, 10);

            assert!(
                gap > 0.03,
                "{gesture} leaves only {gap} m between the thumb and index tips"
            );
        }
    }

    /// Grasp has to read as a full fist, and Point has to keep the index out of it.
    #[test]
    fn grasp_and_point_curl() {
        // The server measures a curl from the fingertip to the middle of the metacarpal.
        let curl = |joints: &[Pose; JOINT_COUNT], finger: Finger| {
            let chain = finger.joints();
            let root = (joints[chain[0]].position + joints[chain[1]].position) / 2.0;

            root.distance(joints[chain[chain.len() - 1]].position)
        };

        let grasp = joints("Grasp", Hand::Left);
        for finger in [Finger::Index, Finger::Middle, Finger::Ring, Finger::Little] {
            let reach = curl(&grasp, finger);
            assert!(
                reach < CURL_TRIGGER,
                "grasp leaves the {} finger {reach} m from the palm, needs under {CURL_TRIGGER}",
                JOINT_NAMES[finger.joints()[0]]
            );
        }

        let point = joints("Point", Hand::Left);
        assert!(
            curl(&point, Finger::Index) > 0.07,
            "point does not extend the index finger"
        );

        // A flat hand must not read as a grab on any finger.
        let plank = joints("Plank", Hand::Left);
        for finger in [Finger::Index, Finger::Middle, Finger::Ring, Finger::Little] {
            let reach = curl(&plank, finger);
            assert!(
                reach > 0.07,
                "plank leaves the {} finger only {reach} m from the palm",
                JOINT_NAMES[finger.joints()[0]]
            );
        }
        for finger in [Finger::Middle, Finger::Ring, Finger::Little] {
            let reach = curl(&point, finger);
            assert!(
                reach < CURL_TRIGGER,
                "point leaves the {} finger {reach} m from the palm",
                JOINT_NAMES[finger.joints()[0]]
            );
        }
    }

    /// Splay fans the fingers apart and a fist gathers them again.
    #[test]
    fn spread_fans_the_fingers() {
        let span = |pose: HandPose| {
            let joints = skeleton(&pose, Hand::Left, 0.185);
            distance(&joints, 10, 25)
        };

        let together = span(HandPose::FLAT);
        let apart = span(HandPose {
            spread: 1.0,
            ..HandPose::FLAT
        });

        assert!(
            apart > together + 0.02,
            "splaying only widened the hand from {together} to {apart}"
        );

        let fist_apart = span(HandPose {
            curl: [1.0; 5],
            spread: 1.0,
            thumb_opposition: 0.5,
        });
        let fist_together = span(HandPose {
            curl: [1.0; 5],
            spread: 0.0,
            thumb_opposition: 0.5,
        });

        assert!(
            fist_apart - fist_together < (apart - together) / 2.0,
            "a fist fans as much as an open hand"
        );
    }

    /// The palm and the wrist are the frames the OpenXR specification defines, which is what lets
    /// the joints go to the server without conversion — and what makes the driver's own hand
    /// tracking offsets land where they were tuned to.
    #[test]
    fn palm_and_wrist_match_the_spec() {
        let flat = skeleton(&HandPose::FLAT, Hand::Left, 0.185);
        let chain = Finger::Middle.joints();
        let base = flat[chain[0]].position;
        let knuckle = flat[chain[1]].position;

        // "The palm joint is located at the center of the middle finger's metacarpal bone."
        let centre = (base + knuckle) / 2.0;
        assert!(
            centre.length() < 1e-4,
            "the palm is not at the centre of the middle metacarpal: {centre:?}"
        );

        // "The backward (+Z) direction is parallel to the middle finger's metacarpal bone, and
        // points away from the finger tips."
        let along = (knuckle - base).normalize();
        assert!(
            along.distance(Vec3::NEG_Z) < 1e-4,
            "the middle metacarpal does not lie on the palm's -Z axis: {along:?}"
        );

        assert!(
            flat[WRIST].position.z > base.z,
            "the wrist is not behind the metacarpals"
        );
        assert!(
            flat[WRIST].orientation.angle_between(Quat::IDENTITY) < 1e-4,
            "the wrist is not oriented like the palm"
        );
    }

    /// Exactly on the axis makes SteamVR's wrist snap; see `skeleton`.
    #[test]
    fn the_middle_metacarpal_is_off_the_wrist_axis() {
        for (hand, name) in [(Hand::Left, "left"), (Hand::Right, "right")] {
            let flat = skeleton(&HandPose::FLAT, hand, 0.185);
            let wrist = flat[WRIST];
            let base = flat[Finger::Middle.joints()[0]].position;
            let towards = wrist.orientation.conjugate() * (base - wrist.position);
            let off = towards.angle_between(Vec3::NEG_Z).to_degrees();
            assert!(
                (3.0..10.0).contains(&off),
                "{name}: the middle metacarpal is {off:.2}° off the wrist's axis"
            );
        }
    }

    /// A fingerless glove model is two-toned at its cuff, which sits at the second knuckle.
    #[test]
    fn the_glove_cuff_is_the_second_knuckle() {
        assert!(!beyond_glove_cuff(PALM) && !beyond_glove_cuff(WRIST));

        for finger in Finger::ALL {
            let chain = finger.joints();

            assert!(!beyond_glove_cuff(chain[0]), "metacarpals are covered");
            assert!(!beyond_glove_cuff(chain[1]), "proximals are covered");

            for joint in chain.iter().skip(2) {
                assert!(
                    beyond_glove_cuff(*joint),
                    "{} should be bare",
                    JOINT_NAMES[*joint]
                );
            }
        }
    }

    /// The built-in gestures resolve against the built-in poses, and a Click really does pass
    /// through a pinch and come back — sampling it at its own keyframes is what a hand sent on the
    /// wire will be doing, so the click has to be inside the recogniser's distance at the middle.
    #[test]
    fn click_passes_through_a_pinch() {
        let settings = settings();

        assert_eq!(
            settings.gestures.len(),
            2,
            "every built-in gesture should have resolved its poses"
        );

        let click = settings
            .find_gesture("Click")
            .map(|index| &settings.gestures[index])
            .expect("Click exists");

        let gap = |phase: f32| {
            let pose = click.sample(&settings.poses, phase);
            let joints = skeleton(&pose, Hand::Left, settings.hand_length);

            distance(&joints, 5, 10)
        };

        assert!(
            gap(0.5) < PINCH_CLICK,
            "the middle of a click is {} m from a pinch",
            gap(0.5)
        );
        assert!(gap(0.0) > 0.03, "a click starts already pinched");
        assert!(gap(1.0) > 0.03, "a click ends still pinched");

        // And it comes back to the pose it started from, so clicking twice works.
        assert_eq!(click.final_pose(), settings.find_pose("Point"));
    }

    /// A gesture naming a pose that does not exist is dropped, rather than played as a flat hand.
    #[test]
    fn a_gesture_with_an_unknown_pose_is_dropped() {
        let mut file = default_file();
        file.gestures.push(GestureEntry {
            name: "Broken".into(),
            description: "names a pose that is not there".into(),
            duration_seconds: 0.5,
            keyframes: vec![KeyframeEntry {
                pose: "Nonexistent".into(),
                phase: 0.0,
            }],
        });

        let settings = resolve(file, Path::new("."));

        assert!(settings.find_gesture("Broken").is_none());
        assert_eq!(settings.gestures.len(), 2);
    }

    /// Joint names round-trip through the matching the model loader uses.
    #[test]
    fn joint_names_match_rig_conventions() {
        let overrides = HashMap::new();

        assert_eq!(joint_from_name("Index_Proximal_L", &overrides), Some(7));
        assert_eq!(joint_from_name("index-proximal", &overrides), Some(7));
        assert_eq!(joint_from_name("Wrist_R", &overrides), Some(WRIST));
        assert_eq!(joint_from_name("Palm_L", &overrides), Some(PALM));
        assert_eq!(joint_from_name("forearm", &overrides), None);

        let overrides = HashMap::from([("Bone.004".to_owned(), "index_tip".to_owned())]);
        assert_eq!(joint_from_name("Bone.004", &overrides), Some(10));
    }
}
