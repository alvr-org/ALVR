#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

pub struct PlatformStrings {
    pub display: String,
    pub device: String,
    pub model: String,
    pub manufacturer: String,
}

pub fn platform_strings() -> PlatformStrings {
    #[cfg(target_os = "android")]
    let display = format!("{} ({})", android::model_name(), android::device_name());
    #[cfg(target_os = "ios")]
    let display = "Apple headset".into();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let display = whoami::devicename();

    #[cfg(target_os = "android")]
    let device = android::device_name();
    #[cfg(not(target_os = "android"))]
    let device = whoami::devicename();

    #[cfg(target_os = "android")]
    let model = android::model_name();
    #[cfg(target_os = "ios")]
    let model = "Apple headset".into();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let model = whoami::hostname();

    #[cfg(target_os = "android")]
    let manufacturer = android::manufacturer_name();
    #[cfg(target_vendor = "apple")]
    let manufacturer = "Apple".into();
    #[cfg(not(any(target_os = "android", target_vendor = "apple")))]
    let manufacturer = "Unknown".into();

    PlatformStrings {
        display,
        device,
        model,
        manufacturer,
    }
}

#[cfg(not(target_os = "android"))]
pub fn local_ip() -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    local_ip_address::local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
