use alvr_common::{
    Pose,
    glam::{Quat, Vec3},
};
use alvr_system_info::Platform;

// Most OpenXR runtime, including Meta's one, do not follow perfectly the specification regarding
// controller pose. The Z axis should point down through the center of the controller grip, the X
// axis should go out perpendicular from the palm, and the position should be aligned roughtly with
// the center of the palm.
// https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html
// Note: right controller offsets are calculated from left controller offsets by mirroring along the
// Y-Z plane.
pub fn controller_pose_offset(platform: Platform, is_right_hand: bool) -> Pose {
    const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;

    let left_offset = match platform {
        Platform::Quest1 => Pose {
            position: Vec3::new(-0.013, -0.005, 0.0),
            orientation: Quat::from_rotation_x(-20.0 * DEG_TO_RAD),
        },
        // todo: check Quest 2
        p if p.is_quest() => Pose {
            position: Vec3::new(-0.005, -0.005, 0.00),
            orientation: Quat::from_rotation_x(-15.0 * DEG_TO_RAD),
        },
        Platform::PicoNeo3 => Pose {
            position: Vec3::new(-0.013, -0.035, 0.0),
            orientation: Quat::IDENTITY,
        },
        // todo: check (base) Pico 4
        p if p.is_pico() => Pose {
            position: Vec3::new(-0.01, -0.035, 0.0),
            orientation: Quat::from_rotation_y(6.0 * DEG_TO_RAD)
                * Quat::from_rotation_x(-6.0 * DEG_TO_RAD),
        },
        p if p.is_vive() => Pose {
            position: Vec3::new(0.0, 0.0, -0.02),
            orientation: Quat::IDENTITY,
        },
        Platform::SamsungGalaxyXR => Pose {
            position: Vec3::new(0.0, 0.0, 0.055),
            orientation: Quat::IDENTITY,
        },
        _ => Pose::IDENTITY,
    };

    if is_right_hand {
        let p = left_offset.position;
        let q = left_offset.orientation;

        Pose {
            position: Vec3::new(-p[0], p[1], p[2]),
            orientation: Quat::from_xyzw(-q.x, q.y, q.z, -q.w),
        }
    } else {
        left_offset
    }
}
