#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

use std::fmt::{Display, Formatter};

// Platform of the device. It is used to match the VR runtime and enable features conditionally.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Platform {
    Quest1,
    Quest2,
    Quest3,
    Quest3S,
    QuestPro,
    QuestUnknown,
    PicoNeo3,
    Pico4,
    Pico4Ultra,
    PicoG3,
    PicoUnknown,
    Focus3,
    FocusVision,
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

impl Platform {
    pub const fn is_quest(&self) -> bool {
        matches!(
            self,
            Platform::Quest1
                | Platform::Quest2
                | Platform::Quest3
                | Platform::Quest3S
                | Platform::QuestPro
                | Platform::QuestUnknown
        )
    }

    pub const fn is_pico(&self) -> bool {
        matches!(
            self,
            Platform::PicoG3
                | Platform::PicoNeo3
                | Platform::Pico4
                | Platform::Pico4Ultra
                | Platform::PicoUnknown
        )
    }

    pub const fn is_vive(&self) -> bool {
        matches!(
            self,
            Platform::Focus3 | Platform::FocusVision | Platform::XRElite | Platform::ViveUnknown
        )
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Platform::Quest1 => "Quest 1",
            Platform::Quest2 => "Quest 2",
            Platform::Quest3 => "Quest 3",
            Platform::Quest3S => "Quest 3S",
            Platform::QuestPro => "Quest Pro",
            Platform::QuestUnknown => "Quest (unknown)",
            Platform::PicoNeo3 => "Pico Neo 3",
            Platform::Pico4 => "Pico 4",
            Platform::Pico4Ultra => "Pico 4 Ultra",
            Platform::PicoG3 => "Pico G3",
            Platform::PicoUnknown => "Pico (unknown)",
            Platform::Focus3 => "VIVE Focus 3",
            Platform::FocusVision => "VIVE Focus Vision",
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

        alvr_common::info!("manufacturer: {manufacturer}, model: {model}, device: {device}");

        match (manufacturer.as_str(), model.as_str(), device.as_str()) {
            ("Oculus", _, "monterey") => Platform::Quest1,
            ("Oculus", _, "hollywood") => Platform::Quest2,
            ("Oculus", _, "eureka") => Platform::Quest3,
            ("Oculus", _, "panther") => Platform::Quest3S,
            ("Oculus", _, "seacliff") => Platform::QuestPro,
            ("Oculus", _, _) => Platform::QuestUnknown,
            ("Pico", "Pico Neo 3" | "Pico Neo3 Link", _) => Platform::PicoNeo3,
            ("Pico", _, "PICOA8110" | "phoenix") => Platform::Pico4,
            ("Pico", _, "sparrow") => Platform::Pico4Ultra,
            ("Pico", _, "merline") => Platform::PicoG3,
            ("Pico", _, _) => Platform::PicoUnknown,
            ("HTC", "VIVE Focus 3", _) => Platform::Focus3,
            ("HTC", "VIVE Focus Vision", _) => Platform::FocusVision,
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
