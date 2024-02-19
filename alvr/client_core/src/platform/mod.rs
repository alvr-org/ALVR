#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn model_name() -> String {
    whoami::hostname()
}
#[cfg(target_os = "ios")]
pub fn model_name() -> String {
    "Apple headset".into()
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn display_name() -> String {
    whoami::devicename()
}
#[cfg(target_os = "android")]
pub fn display_name() -> String {
    android::model_name()
}
#[cfg(target_os = "ios")]
pub fn display_name() -> String {
    "Apple headset".into()
}

#[cfg(not(any(target_os = "android", target_vendor = "apple")))]
pub fn manufacturer_name() -> String {
    "Unknown".into()
}
#[cfg(target_vendor = "apple")]
pub fn manufacturer_name() -> String {
    "Apple".into()
}

#[cfg(not(target_os = "android"))]
pub fn local_ip() -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    local_ip_address::local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
