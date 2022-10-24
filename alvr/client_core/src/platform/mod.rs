#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::{
    context, device_name, try_get_microphone_permission, video_decoder_split, vm, DequeuedFrame,
    VideoDecoderDequeuer, VideoDecoderEnqueuer,
};

#[cfg(not(target_os = "android"))]
pub fn device_name() -> String {
    "Wired headset".into()
}
