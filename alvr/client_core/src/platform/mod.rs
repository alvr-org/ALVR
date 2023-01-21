#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::{
    acquire_wifi_lock, battery_status, context, device_model, local_ip, manufacturer_name,
    release_wifi_lock, try_get_microphone_permission, video_decoder_split, vm,
    VideoDecoderDequeuer, VideoDecoderEnqueuer,
};

#[cfg(not(target_os = "android"))]
pub fn device_model() -> String {
    "Wired headset".into()
}

#[cfg(not(target_os = "android"))]
pub fn manufacturer_name() -> String {
    "Unknown".into()
}

#[cfg(not(any(target_os = "android", target_os = "macos")))]
pub fn local_ip() -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    local_ip_address::local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

#[cfg(target_os = "macos")]
pub fn local_ip() -> std::net::IpAddr {
    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
}

// Return (percentage, is plugged)
#[cfg(not(target_os = "android"))]
pub fn battery_status() -> (f32, bool) {
    (1.0, true)
}
