// Eye and face tracking support for Phoenix devices.
// Supports: Pico 4 Pro And Pico 4 Enterprise.
//
// Eye dilation and seperate gaze is not supported on Pico 4 Pro, unless prop `ro.pxr.externalfunc` is set to 1.
use crate::extra_extensions::get_instance_proc;
use openxr::{
    self as xr,
    sys::{self},
};

const PICO_FACE_EXPRESSION_COUNT: usize = 52;
const TRACKING_MODE_EYE_BIT: u64 = 0x00000004;
const TRACKING_MODE_FACE_BIT: u64 = 0x00000008;

#[repr(C)]
pub struct EyeTrackingDataPICO {
    timestamp: xr::Time,
    left_eye_pose_status: i32, // Bit field (pvrEyePoseStatus) indicating left eye pose status
    right_eye_pose_status: i32, // Bit field (pvrEyePoseStatus) indicating right eye pose status
    combined_eye_pose_status: i32, // Bit field (pvrEyePoseStatus) indicating combined eye pose status

    left_eye_gaze_point: [f32; 3],     // Left Eye Gaze Point
    right_eye_gaze_point: [f32; 3],    // Right Eye Gaze Point
    combined_eye_gaze_point: [f32; 3], // Combined Eye Gaze Point (HMD center-eye point)

    left_eye_gaze_vector: [f32; 3],     // Left Eye Gaze Point
    right_eye_gaze_vector: [f32; 3],    // Right Eye Gaze Point
    combined_eye_gaze_vector: [f32; 3], // Comnbined Eye Gaze Vector (HMD center-eye point)

    left_eye_openness: f32, // Left eye value between 0.0 and 1.0 where 1.0 means fully open and 0.0 closed.
    right_eye_openness: f32, // Right eye value between 0.0 and 1.0 where 1.0 means fully open and 0.0 closed.

    left_eye_pupil_dilation: f32, // Left eye value in millimeters indicating the pupil dilation
    right_eye_pupil_dilation: f32, // Right eye value in millimeters indicating the pupil dilation

    left_eye_position_guide: [f32; 3], // Position of the inner corner of the left eye in meters from the HMD center-eye coordinate system's origin.
    right_eye_position_guide: [f32; 3], // Position of the inner corner of the right eye in meters from the HMD center-eye coordinate system's origin.

    foveated_gaze_direction: [f32; 3], // Position of the gaze direction in meters from the HMD center-eye coordinate system's origin.
    foveated_gaze_tracking_state: i32, // The current state of the foveatedGazeDirection signal.
}

#[repr(C)]
pub struct FaceTrackingDataPICO {
    timestamp: xr::Time,
    blend_shape_weight: [f32; 72], // Only supported up to and including blendshape 51.
    // Float 0 stays 1, Float 1 specifies if the camera can see the face or not. Other floats are not filled.
    video_input_valid: [f32; 10],
    laughing_prob: f32,      // Not filled
    emotion_prob: [f32; 10], // Not filled
}

// xrFunctions
type StartEyeTrackingPICO = unsafe extern "system" fn(sys::Session) -> sys::Result;
type StopEyeTrackingPICO = unsafe extern "system" fn(sys::Session, u64) -> sys::Result;
type SetTrackingModePICO = unsafe extern "system" fn(sys::Session, u64) -> sys::Result;

type GetEyeTrackingDataPICO =
    unsafe extern "system" fn(sys::Session, sys::Time, *mut EyeTrackingDataPICO) -> sys::Result;
type GetFaceTrackingDataPICO = unsafe extern "system" fn(
    sys::Session,
    sys::Time,
    i32,
    *mut FaceTrackingDataPICO,
) -> sys::Result;

pub struct FaceTrackerPhoenix {
    _session: xr::Session<xr::AnyGraphics>,

    start_eye_tracking: StartEyeTrackingPICO,
    stop_eye_tracking: StopEyeTrackingPICO,
    set_tracking_mode: SetTrackingModePICO,

    get_eye_tracking_data: GetEyeTrackingDataPICO,
    get_face_tracking_data: GetFaceTrackingDataPICO,
}

impl FaceTrackerPhoenix {
    pub fn new<G>(session: xr::Session<G>, _: xr::SystemId) -> xr::Result<Self> {
        session
            .instance()
            .exts()
            .ext_eye_gaze_interaction
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let start_eye_tracking = get_instance_proc(&session, "xrStartEyeTrackingPICO")?;
        let stop_eye_tracking = get_instance_proc(&session, "xrStopEyeTrackingPICO")?;
        let set_tracking_mode = get_instance_proc(&session, "xrSetTrackingModePICO")?;

        let get_eye_tracking_data = get_instance_proc(&session, "xrGetEyeTrackingDataPICO")?;
        let get_face_tracking_data = get_instance_proc(&session, "xrGetFaceTrackingDataPICO")?;

        return Ok(Self {
            _session: session.into_any_graphics(),

            start_eye_tracking: start_eye_tracking,
            stop_eye_tracking: stop_eye_tracking,
            set_tracking_mode: set_tracking_mode,

            get_eye_tracking_data: get_eye_tracking_data,
            get_face_tracking_data: get_face_tracking_data,
        });
    }

    pub fn get_face_tracking_data(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let mut face_tracking_data = FaceTrackingDataPICO {
            timestamp: xr::Time::from_nanos(0),
            blend_shape_weight: [0.0; 72],
            video_input_valid: [0.0; 10],
            laughing_prob: 0.0,
            emotion_prob: [0.0; 10],
        };

        unsafe {
            super::xr_res((self.get_face_tracking_data)(
                self._session.as_raw(),
                time,
                0,
                &mut face_tracking_data,
            ))?;
        }

        if face_tracking_data.timestamp.as_nanos() != 0
            && face_tracking_data.video_input_valid[1] == 1.0
        {
            Ok(Some(
                face_tracking_data.blend_shape_weight[..PICO_FACE_EXPRESSION_COUNT].to_vec(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn get_eye_tracking_data(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let mut eye_tracking_data = EyeTrackingDataPICO {
            timestamp: xr::Time::from_nanos(0),
            left_eye_pose_status: 0,
            right_eye_pose_status: 0,
            combined_eye_pose_status: 0,

            left_eye_gaze_point: [0.0; 3],
            right_eye_gaze_point: [0.0; 3],
            combined_eye_gaze_point: [0.0; 3],

            left_eye_gaze_vector: [0.0; 3],
            right_eye_gaze_vector: [0.0; 3],
            combined_eye_gaze_vector: [0.0; 3],

            left_eye_openness: 0.0,
            right_eye_openness: 0.0,

            left_eye_pupil_dilation: 0.0,
            right_eye_pupil_dilation: 0.0,

            left_eye_position_guide: [0.0; 3],
            right_eye_position_guide: [0.0; 3],

            foveated_gaze_direction: [0.0; 3],
            foveated_gaze_tracking_state: 0,
        };

        unsafe {
            super::xr_res((self.get_eye_tracking_data)(
                self._session.as_raw(),
                time,
                &mut eye_tracking_data,
            ))?;
        }

        if eye_tracking_data.timestamp.as_nanos() != 0 {
            let mut flattened = Vec::with_capacity(8);

            // If seperate eye gaze is not supported, we fallback on the combined gaze vector.
            if eye_tracking_data.left_eye_gaze_vector[2] == 0.0
                && eye_tracking_data.right_eye_gaze_vector[2] == 0.0
            {
                flattened.push(eye_tracking_data.combined_eye_gaze_vector[0]);
                flattened.push(eye_tracking_data.combined_eye_gaze_vector[1]);

                flattened.push(eye_tracking_data.combined_eye_gaze_vector[0]);
                flattened.push(eye_tracking_data.combined_eye_gaze_vector[1]);
            } else {
                flattened.push(eye_tracking_data.left_eye_gaze_vector[0]);
                flattened.push(eye_tracking_data.left_eye_gaze_vector[1]);

                flattened.push(eye_tracking_data.right_eye_gaze_vector[0]);
                flattened.push(eye_tracking_data.right_eye_gaze_vector[1]);
            }

            flattened.push(eye_tracking_data.left_eye_openness);
            flattened.push(eye_tracking_data.right_eye_openness);

            flattened.push(eye_tracking_data.left_eye_pupil_dilation);
            flattened.push(eye_tracking_data.right_eye_pupil_dilation);

            Ok(Some(flattened))
        } else {
            Ok(None)
        }
    }

    pub fn start_tracking(&self) -> xr::Result<()> {
        unsafe {
            super::xr_res((self.start_eye_tracking)(self._session.as_raw()))?;
            super::xr_res((self.set_tracking_mode)(
                self._session.as_raw(),
                TRACKING_MODE_EYE_BIT | TRACKING_MODE_FACE_BIT,
            ))
        }
    }

    pub fn stop_tracking(&self) -> xr::Result<()> {
        unsafe { super::xr_res((self.stop_eye_tracking)(self._session.as_raw(), 0)) }
    }
}

impl Drop for FaceTrackerPhoenix {
    fn drop(&mut self) {
        self.stop_tracking().ok();
    }
}
