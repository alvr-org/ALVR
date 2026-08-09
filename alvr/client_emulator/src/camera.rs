use alvr_common::glam::{EulerRot, Mat4, Quat, Vec3};

/// Interpupillary distance used to derive the two eye poses from the head pose.
pub const IPD: f32 = 0.063;

/// Symmetric half-angle field of view, in radians. Roughly matches a Quest-class headset.
const FOV_HALF_ANGLE: f32 = 45.0_f32.to_radians();

const NEAR_CLIP: f32 = 0.02;
const FAR_CLIP: f32 = 100.0;

const MOVE_SPEED: f32 = 2.0;
const FAST_MULTIPLIER: f32 = 3.0;
const ROLL_SPEED: f32 = 1.2;
const HEIGHT_SPEED: f32 = 1.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

/// Which eye a view is rendered for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Eye {
    Left,
    Right,
}

impl Eye {
    /// Signed lateral offset of this eye from the head centre.
    fn offset(self) -> f32 {
        match self {
            Eye::Left => -IPD / 2.0,
            Eye::Right => IPD / 2.0,
        }
    }
}

/// A first person camera driven by keyboard and mouse.
///
/// Movement is horizontal only: WASD translates in the XZ plane regardless of where the camera is
/// looking, and height is changed separately with Page Up / Page Down. There is no collision, so
/// walking through walls is expected.
#[derive(Clone, Copy)]
pub struct Camera {
    /// Head position in world space. Y is up.
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 1.6, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        }
    }
}

/// Per-frame input for [`Camera::apply_input`]. Decoupled from egui so the camera stays testable.
#[derive(Default)]
pub struct CameraInput {
    /// Positive is forward (the direction the camera faces, flattened to horizontal).
    pub forward: f32,
    /// Positive is right.
    pub right: f32,
    /// Positive rolls clockwise from the viewer's perspective.
    pub roll: f32,
    /// Positive moves up.
    pub height: f32,
    /// Mouse motion in pixels, applied to yaw and pitch. Only set while look is captured.
    pub mouse_delta: (f32, f32),
    pub fast: bool,
}

impl Camera {
    pub fn apply_input(&mut self, input: &CameraInput, delta_seconds: f32) {
        self.yaw -= input.mouse_delta.0 * MOUSE_SENSITIVITY;
        self.pitch = (self.pitch - input.mouse_delta.1 * MOUSE_SENSITIVITY)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

        self.roll += input.roll * ROLL_SPEED * delta_seconds;
        self.yaw = self.yaw.rem_euclid(std::f32::consts::TAU);
        self.roll = self.roll.rem_euclid(std::f32::consts::TAU);

        let speed = MOVE_SPEED
            * delta_seconds
            * if input.fast { FAST_MULTIPLIER } else { 1.0 };

        // Horizontal movement only: derive the basis from yaw alone so looking up or down does not
        // move the camera vertically.
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let forward = Vec3::new(-sin_yaw, 0.0, -cos_yaw);
        let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);

        self.position += (forward * input.forward + right * input.right) * speed;
        self.position.y += input.height * HEIGHT_SPEED * delta_seconds;
    }

    /// Head orientation. Yaw then pitch then roll, matching the order the input applies them.
    pub fn orientation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, self.roll)
    }

    /// World-space pose of one eye, offset laterally from the head by half the IPD.
    pub fn eye_position(&self, eye: Eye) -> Vec3 {
        self.position + self.orientation() * Vec3::new(eye.offset(), 0.0, 0.0)
    }

    /// View matrix for one eye, mapping world space into that eye's view space.
    pub fn view_matrix(&self, eye: Eye) -> Mat4 {
        Mat4::from_rotation_translation(self.orientation(), self.eye_position(eye)).inverse()
    }

    /// Projection matrix for the given aspect ratio.
    ///
    /// wgpu clip space has Z in 0..1, unlike OpenGL's -1..1, so this uses the `_rh` variant that
    /// targets the former. Getting this wrong silently breaks the depth buffer rather than failing.
    pub fn projection_matrix(aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(FOV_HALF_ANGLE * 2.0, aspect_ratio, NEAR_CLIP, FAR_CLIP)
    }

    pub fn near_clip() -> f32 {
        NEAR_CLIP
    }

    pub fn far_clip() -> f32 {
        FAR_CLIP
    }

    /// Symmetric FOV reported to the server.
    ///
    /// `Fov` holds half-angles in radians, not tangents: `alvr_graphics` applies `tan()` to these
    /// when building its projection, so storing tangents here would double-apply it.
    pub fn fov() -> alvr_common::Fov {
        alvr_common::Fov {
            left: -FOV_HALF_ANGLE,
            right: FOV_HALF_ANGLE,
            up: FOV_HALF_ANGLE,
            down: -FOV_HALF_ANGLE,
        }
    }
}
