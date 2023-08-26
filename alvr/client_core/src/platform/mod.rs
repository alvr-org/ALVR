#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

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
