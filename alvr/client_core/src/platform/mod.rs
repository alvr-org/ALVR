#[cfg(target_os = "android")]
pub mod android;

use std::fmt::{Display, Formatter};

#[cfg(target_os = "android")]
pub use android::*;

// Platform of the device. It is used to match the VR runtime and enable features conditionally.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Platform {
    Quest1,
    Quest2,
    Quest3,
    QuestPro,
    QuestUnknown,
    PicoNeo3,
    Pico4,
    Focus3,
    XRElite,
    ViveUnknown,
    Yvr,
    Lynx,
    AndroidUnknown,
    AppleHeadset,
    WindowsPc,
    LinuxPc,
    Macos,
    Unknown,
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Platform::Quest1 => "Quest 1",
            Platform::Quest2 => "Quest 2",
            Platform::Quest3 => "Quest 3",
            Platform::QuestPro => "Quest Pro",
            Platform::QuestUnknown => "Quest (unknown)",
            Platform::PicoNeo3 => "Pico Neo 3",
            Platform::Pico4 => "Pico 4",
            Platform::Focus3 => "VIVE Focus 3",
            Platform::XRElite => "VIVE XR Elite",
            Platform::ViveUnknown => "HTC VIVE (unknown)",
            Platform::Yvr => "YVR",
            Platform::Lynx => "Lynx Headset",
            Platform::AndroidUnknown => "Android (unknown)",
            Platform::AppleHeadset => "Apple Headset",
            Platform::WindowsPc => "Windows PC",
            Platform::LinuxPc => "Linux PC",
            Platform::Macos => "macOS",
            Platform::Unknown => "Unknown",
        };
        write!(f, "{}", name)
    }
}

pub fn platform() -> Platform {
    #[cfg(target_os = "android")]
    {
        let manufacturer = android::manufacturer_name();
        let model = android::model_name();
        let device = android::device_name();

        match (manufacturer.as_str(), model.as_str(), device.as_str()) {
            ("Oculus", _, "monterey") => Platform::Quest1,
            ("Oculus", _, "hollywood") => Platform::Quest2,
            ("Oculus", _, "eureka") => Platform::Quest3,
            ("Oculus", _, "seacliff") => Platform::QuestPro,
            ("Oculus", _, _) => Platform::QuestUnknown,
            ("Pico", "Pico Neo 3", _) => Platform::PicoNeo3,
            ("Pico", _, _) => Platform::Pico4,
            ("HTC", "VIVE Focus 3", _) => Platform::Focus3,
            ("HTC", "VIVE XR Series", _) => Platform::XRElite,
            ("HTC", _, _) => Platform::ViveUnknown,
            ("YVR", _, _) => Platform::Yvr,
            ("Lynx Mixed Reality", _, _) => Platform::Lynx,
            _ => Platform::AndroidUnknown,
        }
    }
    #[cfg(target_os = "ios")]
    {
        Platform::AppleHeadset
    }
    #[cfg(windows)]
    {
        Platform::WindowsPc
    }
    #[cfg(target_os = "linux")]
    {
        Platform::LinuxPc
    }
    #[cfg(target_os = "macos")]
    {
        Platform::Macos
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        windows,
        target_os = "linux",
        target_os = "macos"
    )))]
    {
        Platform::Unknown
    }
}

#[cfg(not(target_os = "android"))]
pub fn local_ip() -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    local_ip_address::local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
