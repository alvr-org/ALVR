use crate::extra_extensions::get_instance_proc;
use openxr::{self as xr, sys};

const TRACKING_MODE_FACE_BIT: u64 = 0x00000008;
const TRACKING_MODE_FACE_LIPSYNC: u64 = 0x00002000;
const TRACKING_MODE_FACE_LIPSYNC_BLEND_SHAPES: u64 = 0x00000100;

#[repr(C)]
struct FaceTrackingDataPICO {
    time: sys::Time,
    blend_shape_weight: [f32; 72],
    is_video_input_valid: [f32; 10],
    laughing_probability: f32,
    emotion_probability: [f32; 10],
    reserved: [f32; 128],
}

type StartEyeTrackingPICO = unsafe extern "system" fn(sys::Session) -> sys::Result;

type StopEyeTrackingPICO = unsafe extern "system" fn(sys::Session, u64) -> sys::Result;

type SetTrackingModePICO = unsafe extern "system" fn(sys::Session, u64) -> sys::Result;

type GetFaceTrackingDataPICO = unsafe extern "system" fn(
    sys::Session,
    sys::Time,
    i32,
    *mut FaceTrackingDataPICO,
) -> sys::Result;

pub struct FaceTrackerPico {
    session: xr::Session<xr::AnyGraphics>,
    tracking_flags: u64,
    start_eye_tracking: StartEyeTrackingPICO,
    stop_eye_tracking: StopEyeTrackingPICO,
    set_tracking_mode: SetTrackingModePICO,
    get_face_tracking_data: GetFaceTrackingDataPICO,
}

impl FaceTrackerPico {
    pub fn new<G>(session: xr::Session<G>, visual: bool, audio: bool) -> xr::Result<Self> {
        session
            .instance()
            .exts()
            .ext_eye_gaze_interaction
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let start_eye_tracking = get_instance_proc(&session, "xrStartEyeTrackingPICO")?;
        let stop_eye_tracking = get_instance_proc(&session, "xrStopEyeTrackingPICO")?;
        let set_tracking_mode = get_instance_proc(&session, "xrSetTrackingModePICO")?;
        let get_face_tracking_data = get_instance_proc(&session, "xrGetFaceTrackingDataPICO")?;

        let mut tracking_flags = 0;

        if visual {
            tracking_flags |= TRACKING_MODE_FACE_BIT;
        }
        if audio {
            tracking_flags |= TRACKING_MODE_FACE_LIPSYNC | TRACKING_MODE_FACE_LIPSYNC_BLEND_SHAPES;
        }

        Ok(Self {
            session: session.into_any_graphics(),
            tracking_flags,
            start_eye_tracking,
            stop_eye_tracking,
            set_tracking_mode,
            get_face_tracking_data,
        })
    }

    pub fn get_face_tracking_data(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let mut face_tracking_data = FaceTrackingDataPICO {
            time: xr::Time::from_nanos(0),
            blend_shape_weight: [0.0; 72],
            is_video_input_valid: [0.0; 10],
            laughing_probability: 0.0,
            emotion_probability: [0.0; 10],
            reserved: [0.0; 128],
        };

        unsafe {
            super::xr_res((self.get_face_tracking_data)(
                self.session.as_raw(),
                time,
                0,
                &mut face_tracking_data,
            ))?;

            if face_tracking_data.time.as_nanos() != 0 {
                Ok(Some(face_tracking_data.blend_shape_weight.to_vec()))
            } else {
                Ok(None)
            }
        }
    }

    pub fn start_face_tracking(&self) -> xr::Result<()> {
        unsafe {
            super::xr_res((self.start_eye_tracking)(self.session.as_raw()))?;
            super::xr_res((self.set_tracking_mode)(
                self.session.as_raw(),
                self.tracking_flags,
            ))
        }
    }

    pub fn stop_face_tracking(&self) -> xr::Result<()> {
        unsafe {
            super::xr_res((self.stop_eye_tracking)(
                self.session.as_raw(),
                self.tracking_flags,
            ))
        }
    }
}
