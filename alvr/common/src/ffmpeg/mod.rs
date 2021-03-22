#![allow(dead_code, unused_variables)]

mod ff {
    include!(concat!(env!("OUT_DIR"), "/ffmpeg_module.rs"));
}

use crate::StrResult;

pub enum FfmpegOptionValue {
    String(String),
    Int(i64),
    Double(f64),
    Rational { num: i32, den: i32 },
    Binary(Vec<u8>),
    Dictionary(Vec<(String, String)>),
}

pub enum FfmpegVideoEncoderType {
    D3D11VA,
    Vulkan,
    Software,
}

pub struct FfmpegVideoEncoderDesc {
    pub encoder_type: FfmpegVideoEncoderType,
    pub name: String,
    pub context_options: Vec<(String, FfmpegOptionValue)>,
    pub priv_data_options: Vec<(String, FfmpegOptionValue)>,
    pub codec_open_options: Vec<(String, String)>,
    pub frame_otpions: Vec<(String, FfmpegOptionValue)>,
    pub vendor_specific_context_options: Vec<(String, String)>,
    pub hw_frames_context_options: Vec<(String, FfmpegOptionValue)>,
}

struct FfmpegEncoder {
    encoder: *mut ff::Encoder,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(
        resolution_width: u32,
        resolution_height: u32,
        fps: f32,
        encoder_desc: FfmpegVideoEncoderDesc,
    ) -> StrResult<Self> {
        todo!()
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {}
}
