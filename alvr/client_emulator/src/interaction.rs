//! OpenXR's interaction poses, and where they sit on an emulated device.
//!
//! An application never sees a hand as 26 joints. It asks for *poses* — where to aim from, where
//! the hand grips, where a fingertip pokes — and the runtime derives those from the joints. Those
//! derivations are where this project's bugs have lived: an aim ray that came out of the middle
//! finger, an aim origin that moved when the index finger curled, a grip that sat 14 cm in front of
//! the palm. None of them are visible in the joints themselves, which is why they were each found
//! by their symptoms rather than by looking.
//!
//! So this module computes the poses as the OpenXR 1.1 specification defines them (§12.36, "Hand
//! interaction profile"), and the toolbar can draw any one of them as an axis gizmo. That makes the
//! derivation observable: if the ray leaves the wrong place, the gizmo shows it leaving the wrong
//! place, in the same frame the hand is drawn in.
//!
//! Everything here is in the **palm frame**, matching [`crate::hands::skeleton`]'s output, so
//! multiplying by the hand's world transform places a gizmo. The spec's own axis conventions are
//! quoted at each pose, because several are counter-intuitive — grip's -Z runs *across* the hand
//! from the little finger to the index, not along it.

use crate::{
    controllers::Hand,
    hands::{self, JOINT_COUNT},
};
use alvr_common::{
    Pose,
    glam::{Quat, Vec3},
};

/// Joint indices this module reads, from `XR_EXT_hand_tracking`'s ordering.
mod joint {
    pub const PALM: usize = 0;
    pub const WRIST: usize = 1;
    pub const THUMB_PROXIMAL: usize = 3;
    pub const THUMB_TIP: usize = 5;
    pub const INDEX_PROXIMAL: usize = 7;
    pub const INDEX_DISTAL: usize = 9;
    #[cfg(test)]
    pub const LITTLE_PROXIMAL: usize = 22;
    pub const INDEX_TIP: usize = 10;
}

/// An orthonormal frame from a forward axis and a rough up, the way every pose below is specified:
/// the spec pins one axis exactly and gives the second as a direction it should "roughly" point.
///
/// `z` is the frame's +Z. `up_hint` is projected perpendicular to it, so it need only be nearly
/// right. Falls back to any perpendicular if the two are parallel, which keeps a degenerate hand
/// from producing a `NaN` gizmo rather than no gizmo.
fn frame(z: Vec3, up_hint: Vec3) -> Quat {
    let z = z.normalize_or(Vec3::Z);
    let y = (up_hint - z * up_hint.dot(z))
        .try_normalize()
        .unwrap_or_else(|| z.any_orthonormal_vector());

    Quat::from_mat3(&alvr_common::glam::Mat3::from_cols(y.cross(z), y, z))
}

/// The poses a tracked hand reports, as the OpenXR specification defines them.
///
/// Deliberately every pose rather than only the ones ALVR currently publishes: the point of the
/// gizmo is to compare where a pose *should* be against where the thing on screen behaves as
/// though it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HandInteraction {
    /// The palm joint itself. Not an interaction pose, but the frame ALVR's driver derives the
    /// SteamVR device pose from, so it is what every offset in the server settings is relative to.
    Palm,
    /// The wrist joint, the root of the skeleton SteamVR is sent.
    Wrist,
    /// `/input/grip/pose` — holding an object in a full-hand grip.
    Grip,
    /// `/input/aim/pose` — pointing at something out of arm's reach.
    Aim,
    /// `/input/poke_ext/pose` — touching and pushing a small object with a fingertip.
    Poke,
    /// `/input/pinch_ext/pose` — manipulating a small object between finger and thumb.
    Pinch,
}

impl HandInteraction {
    pub const ALL: [Self; 6] = [
        Self::Palm,
        Self::Wrist,
        Self::Grip,
        Self::Aim,
        Self::Poke,
        Self::Pinch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Palm => "Palm",
            Self::Wrist => "Wrist",
            Self::Grip => "Grip",
            Self::Aim => "Aim",
            Self::Poke => "Poke",
            Self::Pinch => "Pinch",
        }
    }

    /// Shown as the dropdown entry's tooltip, so the convention is readable without the spec open.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Palm => {
                "The OpenXR palm joint: centre of the middle finger's metacarpal, -Z towards the \
                 fingertips, +Y out of the back of the hand. The driver's device pose is this plus \
                 the hand tracking offsets in the server settings."
            }
            Self::Wrist => "The wrist joint, root of the 31-bone skeleton sent to SteamVR.",
            Self::Grip => {
                "Holding an object in a full grip. On the axis of a held tube, inside the closed \
                 fingers, -Z running across the hand from the little finger to the index, +X to \
                 the user's right for both hands."
            }
            Self::Aim => {
                "Pointing at something out of reach. At the root of the index finger with -Z along \
                 the pointing direction, and stabilised: it must not move when the fingers do, or a \
                 pinch drags the aim off the target as you click it."
            }
            Self::Poke => {
                "Pushing a small object with the fingertip. On the surface of the extended index \
                 fingertip, +Z pointing back down the finger towards the knuckle. Unlike aim, this \
                 one follows the finger."
            }
            Self::Pinch => {
                "Manipulating a small object between finger and thumb. Where the thumb and index \
                 tips meet, +Z pointing back towards the midpoint of their proximal joints."
            }
        }
    }

    /// Where this pose sits, in the palm frame, for a hand in the given articulation.
    ///
    /// `joints` is [`crate::hands::skeleton`]'s output: 26 palm-relative joint poses.
    pub fn transform(self, joints: &[Pose; JOINT_COUNT], hand: Hand) -> Pose {
        // The palm frame's axes, named as the hand sees them. Mirrored on the right hand, where
        // the thumb lies on -X; see the module docs on hands.rs.
        let radial = match hand {
            Hand::Left => Vec3::X,
            Hand::Right => Vec3::NEG_X,
        };
        let dorsal = Vec3::Y;
        let proximal = Vec3::Z;

        match self {
            Self::Palm => joints[joint::PALM],
            Self::Wrist => joints[joint::WRIST],

            // "The position of the grip pose is at the centroid of the user's palm... The Z axis
            // goes through the center of the user's curled fingers, and the -Z direction (forward)
            // goes from the little finger to the index finger... The +X direction points away from
            // the palm of the left hand and into the palm of the right hand", which the spec then
            // restates as +X pointing to the user's right for both hands. The spec's figure puts
            // the origin on the axis of the held tube, inside the closed fingers, not at the palm
            // joint in the middle of the hand; see `hands::fist_centre`.
            Self::Grip => Pose {
                position: hands::fist_centre(joints),
                // +Z away from the index, and +Y towards the wrist, which together put +X out of
                // the palm on the left hand and out of its back on the right — the same direction
                // in the world for both, as the spec requires.
                orientation: frame(-radial, proximal),
            },

            // "The position of an aim pose is typically in front of the user's hand... The -Z
            // direction is the forward direction of the aiming gesture." The spec's figure puts the
            // origin at the root of the index finger, with the ray running out along it, so that
            // is the knuckle. Stabilised, per "the orientation of an aim pose is typically
            // stabilized": the knuckle is rigid with the palm, since the metacarpal does not
            // articulate, and the direction is a constant of the hand, so a pinch cannot move the
            // aim. This mirrors what the driver publishes as SteamVR's /pose/tip; see
            // `server_openvr::tracking::hand_tip_offset`.
            Self::Aim => Pose {
                position: joints[joint::INDEX_PROXIMAL].position,
                orientation: frame(-hands::aim_direction(hand), dorsal),
            },

            // "The position of the poke pose is at the surface of the extended index fingertip...
            // The +Z direction points from the fingertip towards the knuckle and parallel to the
            // index finger distal bone... The poke pose must rotate together with the tip of the
            // finger." The one pose that is *supposed* to follow the articulation. The tip joint
            // already is that surface: the skeleton's distal bone runs to the end of the finger,
            // and the bundled hand's mesh ends 1.4 mm past its own index tip bone, so pushing the
            // pose further out would leave it in the air in front of the finger.
            Self::Poke => {
                let tip = joints[joint::INDEX_TIP].position;

                Pose {
                    position: tip,
                    orientation: frame(
                        joints[joint::INDEX_DISTAL].position - tip,
                        joints[joint::INDEX_TIP].orientation * dorsal,
                    ),
                }
            }

            // "The position of the pinch pose is typically where the index and thumb fingertips
            // will touch each other... The +Z axis is the backward direction, typically the
            // direction from the pinch position pointing to the mid point of thumb and finger
            // proximal joints... the +Y direction for both hands should be roughly pointing up"
            // with the palms facing each other, which is the thumb side of the hand.
            Self::Pinch => {
                let position =
                    (joints[joint::THUMB_TIP].position + joints[joint::INDEX_TIP].position) / 2.0;
                let proximals = (joints[joint::THUMB_PROXIMAL].position
                    + joints[joint::INDEX_PROXIMAL].position)
                    / 2.0;

                Pose {
                    position,
                    orientation: frame(proximals - position, radial),
                }
            }
        }
        .normalized()
    }
}

/// A pose with a unit orientation, since a gizmo built from measured joints accumulates drift.
trait Normalized {
    fn normalized(self) -> Self;
}

impl Normalized for Pose {
    fn normalized(mut self) -> Self {
        self.orientation = self.orientation.normalize();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands::{default_settings, skeleton};

    fn joints(pose: &str, hand: Hand) -> [Pose; JOINT_COUNT] {
        let settings = default_settings();
        let index = settings.find_pose(pose).expect("pose exists");

        skeleton(&settings.pose_at(index), hand, settings.hand_length)
    }

    /// Every pose is a real frame: orthonormal, right-handed, and finite.
    #[test]
    fn poses_are_orthonormal_frames() {
        for hand in Hand::BOTH {
            for pose in ["Idle", "Grasp", "Point", "Pinch"] {
                let joints = joints(pose, hand);

                for interaction in HandInteraction::ALL {
                    let transform = interaction.transform(&joints, hand);
                    let label = interaction.label();

                    assert!(
                        transform.position.is_finite() && transform.orientation.is_finite(),
                        "{label} in {pose} is not finite: {transform:?}"
                    );

                    let x = transform.orientation * Vec3::X;
                    let y = transform.orientation * Vec3::Y;
                    let z = transform.orientation * Vec3::Z;

                    assert!(
                        (x.cross(y) - z).length() < 1e-4,
                        "{label} in {pose} is left-handed"
                    );
                    assert!(
                        x.dot(y).abs() < 1e-4 && y.dot(z).abs() < 1e-4 && z.dot(x).abs() < 1e-4,
                        "{label} in {pose} is not orthogonal"
                    );
                }
            }
        }
    }

    /// The spec pins two axes in the world rather than in the hand, and they are the ones easiest
    /// to get backwards: grip's +X and pinch's +Y both point the *same* way for the two hands,
    /// where every other axis mirrors.
    #[test]
    fn world_pinned_axes_agree_between_hands() {
        let left = joints("Pinch", Hand::Left);
        let right = joints("Pinch", Hand::Right);

        // The two hands are mirror images, so how an axis relates between them says which kind of
        // direction the spec pinned it to. A world direction that *flips* under the mirror — "to
        // the user's right" — must come back negated; one that survives it — "up" — must come back
        // unchanged. Getting these two backwards is the easy mistake, and it is invisible on one
        // hand alone.
        let grip_left = HandInteraction::Grip
            .transform(&left, Hand::Left)
            .orientation;
        let grip_right = HandInteraction::Grip
            .transform(&right, Hand::Right)
            .orientation;

        let mirror = |v: Vec3| Vec3::new(-v.x, v.y, v.z);

        assert!(
            (grip_left * Vec3::X + mirror(grip_right * Vec3::X)).length() < 1e-4,
            "grip +X should point the same way in the world for both hands"
        );

        let pinch_left = HandInteraction::Pinch
            .transform(&left, Hand::Left)
            .orientation;
        let pinch_right = HandInteraction::Pinch
            .transform(&right, Hand::Right)
            .orientation;

        // Unlike grip's +X, pinch's +Y is pinned to "up", which the mirror leaves alone.
        assert!(
            (pinch_left * Vec3::Y - mirror(pinch_right * Vec3::Y)).length() < 1e-4,
            "pinch +Y should point up for both hands"
        );
    }

    /// The spec's "Example grip pose" figure: fists held in front of the body, thumbs up, palms
    /// facing each other, around a vertical tube. There +X points to the user's right and Z runs
    /// down the tube for both hands, which leaves +Y pointing back at the user.
    #[test]
    fn grip_matches_the_spec_figure() {
        for hand in Hand::BOTH {
            // The palm frame's axes in the world for that hold. Fingers forward, so the palm's +Z
            // (towards the wrist) points back at the user; the back of the hand faces outwards;
            // and the thumb, on the palm's +X for the left hand and -X for the right, points up.
            let (x, y) = match hand {
                Hand::Left => (Vec3::Y, Vec3::NEG_X),
                Hand::Right => (Vec3::NEG_Y, Vec3::X),
            };
            let palm = Quat::from_mat3(&alvr_common::glam::Mat3::from_cols(x, y, Vec3::Z));
            let joints = joints("Grasp", hand);

            // Anchors the hold in the skeleton itself rather than in this module's idea of which
            // side the thumb is on: if that were mirrored, this hold would put the index knuckle
            // below the little finger's. (Not the thumb tip, which a fist wraps across the front.)
            let height = |index: usize| (palm * joints[index].position).y;
            assert!(
                height(joint::INDEX_PROXIMAL) > height(joint::LITTLE_PROXIMAL) + 0.04,
                "{} hand is upside down in the figure's hold",
                hand.side()
            );

            let grip = palm * HandInteraction::Grip.transform(&joints, hand).orientation;

            for (axis, actual, expected) in [
                ("+X", grip * Vec3::X, Vec3::X),
                ("+Y", grip * Vec3::Y, Vec3::Z),
                ("+Z", grip * Vec3::Z, Vec3::NEG_Y),
            ] {
                assert!(
                    actual.dot(expected) > 0.99,
                    "{} grip {axis} is {actual:?}, the figure has {expected:?}",
                    hand.side()
                );
            }
        }
    }

    /// The figure's other half: the grip sits on the held tube's axis, inside the closed fist —
    /// on the palm side of the palm joint, and ringed by the curled middle finger.
    #[test]
    fn grip_is_inside_the_fist() {
        const MIDDLE_LOOP: [usize; 5] = [joint::PALM, 12, 13, 14, 15];

        for hand in Hand::BOTH {
            let joints = joints("Grasp", hand);
            let grip = HandInteraction::Grip.transform(&joints, hand).position;
            let offset = grip - joints[joint::PALM].position;

            println!(
                "{} grip is {:.1} mm palmar and {:.1} mm distal of the palm joint",
                hand.side(),
                -offset.y * 1000.0,
                -offset.z * 1000.0
            );

            assert!(
                offset.y < -0.015,
                "grip is not on the palm side of the hand"
            );

            for joint in MIDDLE_LOOP {
                let distance = grip.distance(joints[joint].position);
                assert!(
                    distance < 0.045,
                    "joint {joint} is {:.1} mm from the grip, so the fist does not ring it",
                    distance * 1000.0
                );
            }
        }
    }

    /// Aim is the pose that must not move with the fingers, and poke is the one that must.
    #[test]
    fn aim_is_stable_and_poke_follows_the_finger() {
        for hand in Hand::BOTH {
            let reference = HandInteraction::Aim.transform(&joints("Point", hand), hand);

            for pose in ["Idle", "Grasp", "Point", "Pinch"] {
                let joints = joints(pose, hand);
                let aim = HandInteraction::Aim.transform(&joints, hand);

                assert!(
                    aim.position.distance(reference.position) < 1e-4,
                    "{pose}: aim origin moved {:.1} mm",
                    aim.position.distance(reference.position) * 1000.0
                );
                assert!(
                    (aim.orientation * Vec3::NEG_Z).dot(reference.orientation * Vec3::NEG_Z)
                        > 0.9999,
                    "{pose}: aim direction turned"
                );
            }

            let pointing = HandInteraction::Poke.transform(&joints("Point", hand), hand);
            let pinching = HandInteraction::Poke.transform(&joints("Pinch", hand), hand);

            assert!(
                pointing.position.distance(pinching.position) > 0.02,
                "poke should follow the fingertip, but barely moved between pointing and pinching"
            );
        }
    }

    /// Pointing is when aim has to be right: the ray leaves the root of the index finger, as in the
    /// spec's figure, and runs out along the extended finger, so the fingertip lies on it.
    #[test]
    fn aim_runs_along_the_finger_when_pointing() {
        for hand in Hand::BOTH {
            let joints = joints("Point", hand);
            let aim = HandInteraction::Aim.transform(&joints, hand);
            let direction = aim.orientation * Vec3::NEG_Z;

            let to_tip = joints[joint::INDEX_TIP].position - aim.position;
            let along = to_tip.dot(direction);
            let off_ray = (to_tip - direction * along).length();

            assert!(along > 0.05, "fingertip is not ahead of the aim origin");
            assert!(
                off_ray < 0.01,
                "fingertip is {:.1} mm off the aim ray",
                off_ray * 1000.0
            );
        }
    }
}
