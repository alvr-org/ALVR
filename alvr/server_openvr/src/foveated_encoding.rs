use alvr_common::{
    AlvrFoveatedEncodingParams, ViewParams,
    glam::{Quat, UVec2, Vec2, Vec3},
};
use alvr_server_core::align_foveation_center_shift;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const GAZE_FILTER_TIME_CONSTANT: Duration = Duration::from_millis(30);
const INVALID_GAZE_HOLD_DURATION: Duration = Duration::from_secs(1);
// Maximum retained center entries: tracking updates, not video frames.
const CENTER_HISTORY_CAPACITY: usize = 360;
const CENTER_BOUNDARY_EPSILON: f32 = 1e-5;

pub struct EyeTrackedFoveation {
    pub view_params: Option<[ViewParams; 2]>,
    params: AlvrFoveatedEncodingParams,
    // Total non-central extent on each axis, in pixels (both edges combined).
    edge_size: Vec2,
    last_update_timestamp: Option<Duration>,
    // Tracking timestamp, server event-processing time, and filtered head-local direction.
    last_valid_sample: Option<(Duration, Instant, Vec3)>,
    center_history: VecDeque<(Duration, [[f32; 2]; 2])>,
}

impl EyeTrackedFoveation {
    pub fn new(params: AlvrFoveatedEncodingParams, view_resolution: UVec2) -> Self {
        let resolution = view_resolution.as_vec2();

        Self {
            view_params: None,
            params,
            edge_size: resolution - Vec2::from_array(params.center_size) * resolution,
            last_update_timestamp: None,
            last_valid_sample: None,
            center_history: VecDeque::new(),
        }
    }

    /// Update from head-local gaze at the tracking `poll_timestamp`, used for filtering and history.
    /// `now` is the server's monotonic time when handling the event. Either clock can expire a
    /// held sample; missing or invalid gaze does not refresh its age.
    pub fn update(&mut self, timestamp: Duration, orientation: Option<Quat>, now: Instant) {
        // Duplicate or reordered tracking packets must not rewind or refresh the filter.
        if self
            .last_update_timestamp
            .is_some_and(|previous| timestamp <= previous)
        {
            return;
        }
        self.last_update_timestamp = Some(timestamp);

        if self
            .last_valid_sample
            .is_some_and(|(sample_timestamp, processed_at, _)| {
                timestamp.saturating_sub(sample_timestamp) > INVALID_GAZE_HOLD_DURATION
                    || now.saturating_duration_since(processed_at) > INVALID_GAZE_HOLD_DURATION
            })
        {
            self.last_valid_sample = None;
        }

        if let Some(orientation) = orientation
            && orientation.is_finite()
            && orientation.length_squared().is_finite()
            && orientation.length_squared() > f32::EPSILON
        {
            let direction = orientation.normalize() * Vec3::NEG_Z;
            if direction.z < -f32::EPSILON {
                let direction =
                    if let Some((previous_timestamp, _, previous)) = self.last_valid_sample {
                        let delta = timestamp.saturating_sub(previous_timestamp).as_secs_f32();
                        let alpha = 1.0 - (-delta / GAZE_FILTER_TIME_CONSTANT.as_secs_f32()).exp();

                        previous.lerp(direction, alpha).normalize()
                    } else {
                        direction
                    };
                self.last_valid_sample = Some((timestamp, now, direction));
            }
        }

        let Some((_, _, direction)) = self.last_valid_sample else {
            return;
        };
        // Wait for this client's real eye poses and FOV, including after a reconnect.
        let Some(view_params) = self.view_params else {
            return;
        };
        let center_size = Vec2::from_array(self.params.center_size);
        let [left_view, right_view] = view_params;
        let (Some(left), Some(right)) = (
            project_gaze(direction, left_view, center_size),
            project_gaze(direction, right_view, center_size),
        ) else {
            return;
        };
        let mut centers = [left, right];

        if centers
            .iter()
            .any(|center| center.abs().cmpgt(Vec2::ONE).any())
        {
            // Constrain the shared ray before projecting it again. Clamping each eye alone can
            // move their high-density regions to different visual directions at the FOV edges.
            let mut boundaries = [(Vec2::ZERO, 0.0); 8];
            // Work on the head-local z = -1 plane, correcting only X and Y.
            let ray = direction / -direction.z;
            for (boundaries, view) in boundaries.chunks_exact_mut(4).zip(view_params) {
                let lower = Vec2::new(view.fov.left.tan(), view.fov.down.tan());
                let upper = Vec2::new(view.fov.right.tan(), view.fov.up.tan());
                // Inset the FOV so the entire central region stays within the view.
                let margin = (upper - lower) * center_size * 0.5;
                let lower = lower + margin;
                let upper = upper - margin;

                for (boundary, normal) in boundaries.iter_mut().zip([
                    Vec3::new(1.0, 0.0, lower.x),
                    Vec3::new(-1.0, 0.0, -upper.x),
                    Vec3::new(0.0, 1.0, lower.y),
                    Vec3::new(0.0, -1.0, -upper.y),
                ]) {
                    let normal = view.pose.orientation * normal;
                    // normal.dot(ray + (dx, dy, 0)) >= 0 becomes normal.xy.dot(correction) >= offset.
                    // offset = -normal.dot(ray) is not a distance: the normal is not normalized.
                    *boundary = (normal.truncate(), -normal.dot(ray));
                }
            }

            let mut maybe_correction: Option<Vec2> = None;
            let mut consider_correction = |candidate: Vec2| {
                if candidate.is_finite()
                    && boundaries.iter().all(|(normal, offset)| {
                        normal.dot(candidate) + CENTER_BOUNDARY_EPSILON >= *offset
                    })
                    && maybe_correction.is_none_or(|previous| {
                        candidate.length_squared() < previous.length_squared()
                    })
                {
                    maybe_correction = Some(candidate);
                }
            };

            // The current ray is outside the feasible region. Its shortest 2D correction is a
            // projection onto a boundary or a boundary intersection, not a minimum-angle rotation.
            for (index, (normal, offset)) in boundaries.iter().enumerate() {
                if normal.length_squared() > f32::EPSILON {
                    consider_correction(*normal * (*offset / normal.length_squared()));
                }
                for (other_normal, other_offset) in boundaries.iter().skip(index + 1) {
                    let determinant = normal.perp_dot(*other_normal);
                    if determinant.abs() > f32::EPSILON {
                        consider_correction(Vec2::new(
                            (offset * other_normal.y - normal.y * other_offset) / determinant,
                            (normal.x * other_offset - offset * other_normal.x) / determinant,
                        ));
                    }
                }
            }

            // Fixed-size regions need not have a shared solution. If no correction is usable,
            // keep the original projections; alignment below clamps each eye independently.
            if let Some(correction) = maybe_correction {
                let corrected = ray + correction.extend(0.0);
                if let (Some(left), Some(right)) = (
                    project_gaze(corrected, left_view, center_size),
                    project_gaze(corrected, right_view, center_size),
                ) {
                    centers = [left, right];
                }
            }
        }

        let [left, right] = centers;
        let [edge_ratio_x, edge_ratio_y] = self.params.edge_ratio;
        // Only the right eye's X coordinate is mirrored in the packed encoder texture.
        // Per-eye quantization and the reserved boundary step can shift centers away from the
        // shared direction even when a common correction was found above.
        let centers = [left, Vec2::new(-right.x, right.y)].map(|center| {
            [
                align_foveation_center_shift(center.x, self.edge_size.x, edge_ratio_x),
                align_foveation_center_shift(center.y, self.edge_size.y, edge_ratio_y),
            ]
        });
        self.center_history.push_back((timestamp, centers));
        while self.center_history.len() > CENTER_HISTORY_CAPACITY {
            self.center_history.pop_front();
        }
    }

    /// Return aligned centers only for an exact tracking timestamp still in the bounded history.
    /// Gaze expiry stops reusing the input sample, but preserves stored centers for repeated frames.
    pub fn centers(&self, timestamp: Duration) -> Option<[[f32; 2]; 2]> {
        self.center_history
            .iter()
            .rev()
            .find_map(|(sample_timestamp, centers)| {
                (*sample_timestamp == timestamp).then_some(*centers)
            })
    }
}

// Project a shared head-local direction into one eye; view orientation maps eye-local to head-local.
// Eye positions are unused, so this does not model finite-distance binocular parallax.
// Return center shifts, not UVs, before right-eye X mirroring, alignment, or clamping.
fn project_gaze(direction: Vec3, view: ViewParams, center_size: Vec2) -> Option<Vec2> {
    let direction = view.pose.orientation.inverse() * direction;
    if !direction.is_finite() || direction.z >= -f32::EPSILON {
        return None;
    }

    let lower = Vec2::new(view.fov.left.tan(), view.fov.down.tan());
    let upper = Vec2::new(view.fov.right.tan(), view.fov.up.tan());
    let span = upper - lower;
    if !span.is_finite() || span.cmple(Vec2::splat(f32::EPSILON)).any() {
        return None;
    }
    let tangent = direction.truncate() / -direction.z;
    // Image UVs have Y pointing down, opposite to eye-local Y.
    let uv = Vec2::new(
        (tangent.x - lower.x) / span.x,
        (upper.y - tangent.y) / span.y,
    );
    // center_size is a fraction of the view: shift = (uv - 0.5) * 2 / (1 - center_size).
    // On a movable axis, 0 centers the region and +/-1 touches a view edge; values may exceed this.
    let movable_fraction = Vec2::ONE - center_size;
    let center_shift = Vec2::new(
        if movable_fraction.x > f32::EPSILON {
            (uv.x - 0.5) * 2.0 / movable_fraction.x
        } else {
            0.0
        },
        if movable_fraction.y > f32::EPSILON {
            (uv.y - 0.5) * 2.0 / movable_fraction.y
        } else {
            0.0
        },
    );

    center_shift.is_finite().then_some(center_shift)
}
