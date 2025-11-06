#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

use alvr_common::settings_schema::SettingsSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub const PACKAGE_NAME_STORE: &str = "alvr.client";
pub const PACKAGE_NAME_GITHUB_DEV: &str = "alvr.client.dev";
pub const PACKAGE_NAME_GITHUB_STABLE: &str = "alvr.client.stable";

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
    Pico4Pro,
    Pico4Enterprise,
    Pico4Ultra,
    PicoG3,
    PicoUnknown,
    Focus3,
    FocusVision,
    XRElite,
    ViveUnknown,
    Yvr,
    PlayForDreamMR,
    Lynx,
    SamsungGalaxyXR,
    AndroidUnknown,
    VisionOSHeadset,
    WindowsPc,
    LinuxPc,
    Macos,
    Unknown,
}

impl Platform {
    pub const fn is_quest(&self) -> bool {
        matches!(
            self,
            Self::Quest1
                | Self::Quest2
                | Self::Quest3
                | Self::Quest3S
                | Self::QuestPro
                | Self::QuestUnknown
        )
    }

    pub const fn is_pico(&self) -> bool {
        matches!(
            self,
            Self::PicoG3
                | Self::PicoNeo3
                | Self::Pico4
                | Self::Pico4Pro
                | Self::Pico4Enterprise
                | Self::Pico4Ultra
                | Self::PicoUnknown
        )
    }

    pub const fn is_vive(&self) -> bool {
        matches!(
            self,
            Self::Focus3 | Self::FocusVision | Self::XRElite | Self::ViveUnknown
        )
    }

    pub const fn is_yvr(&self) -> bool {
        matches!(self, Self::Yvr | Self::PlayForDreamMR)
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Quest1 => "Quest 1",
            Self::Quest2 => "Quest 2",
            Self::Quest3 => "Quest 3",
            Self::Quest3S => "Quest 3S",
            Self::QuestPro => "Quest Pro",
            Self::QuestUnknown => "Quest (unknown)",
            Self::PicoNeo3 => "Pico Neo 3",
            Self::Pico4 => "Pico 4",
            Self::Pico4Pro => "Pico 4 Pro",
            Self::Pico4Enterprise => "Pico 4 Enterprise",
            Self::Pico4Ultra => "Pico 4 Ultra",
            Self::PicoG3 => "Pico G3",
            Self::PicoUnknown => "Pico (unknown)",
            Self::Focus3 => "VIVE Focus 3",
            Self::FocusVision => "VIVE Focus Vision",
            Self::XRElite => "VIVE XR Elite",
            Self::ViveUnknown => "HTC VIVE (unknown)",
            Self::Yvr => "YVR",
            Self::PlayForDreamMR => "Play For Dream MR",
            Self::Lynx => "Lynx Headset",
            Self::SamsungGalaxyXR => "Samsung Galaxy XR",
            Self::AndroidUnknown => "Android (unknown)",
            Self::VisionOSHeadset => "visionOS Headset",
            Self::WindowsPc => "Windows PC",
            Self::LinuxPc => "Linux PC",
            Self::Macos => "macOS",
            Self::Unknown => "Unknown",
        };
        write!(f, "{name}")
    }
}

#[cfg_attr(not(target_os = "android"), expect(unused_variables))]
pub fn platform(runtime_name: Option<String>, runtime_version: Option<u64>) -> Platform {
    #[cfg(target_os = "android")]
    {
        let manufacturer = android::manufacturer_name();
        let model = android::model_name();
        let device = android::device_name();
        let product = android::product_name();

        // TODO: Better Android XR heuristic
        // (Maybe check runtime json for /system/lib64/libopenxr.google.so?)

        alvr_common::info!(
            "manufacturer: {manufacturer}, model: {model}, device: {device}, product: {product}, \
            runtime_name: {runtime_name:?}, runtime_version: {runtime_version:?}",
        );

        match (
            manufacturer.as_str(),
            model.as_str(),
            device.as_str(),
            product.as_str(),
        ) {
            ("Oculus", _, "monterey", _) => Platform::Quest1,
            ("Oculus", _, "hollywood", _) => Platform::Quest2,
            ("Oculus", _, "eureka", _) => Platform::Quest3,
            ("Oculus", _, "panther", _) => Platform::Quest3S,
            ("Oculus", _, "seacliff", _) => Platform::QuestPro,
            ("Oculus", _, _, _) => Platform::QuestUnknown,
            ("Pico", "Pico Neo 3" | "Pico Neo3 Link", _, _) => Platform::PicoNeo3,
            ("Pico", _, _, "PICO 4 Pro") => Platform::Pico4Pro,
            ("Pico", _, _, "PICO 4 Enterprise") => Platform::Pico4Enterprise,
            ("Pico", _, _, "PICO 4") => Platform::Pico4,
            ("Pico", _, _, "PICO 4 Ultra") => Platform::Pico4Ultra,
            ("Pico", _, _, "PICO G3") => Platform::PicoG3,
            ("Pico", _, _, _) => Platform::PicoUnknown,
            ("HTC", "VIVE Focus 3", _, _) => Platform::Focus3,
            ("HTC", "VIVE Focus Vision", _, _) => Platform::FocusVision,
            ("HTC", "VIVE XR Series", _, _) => Platform::XRElite,
            ("HTC", _, _, _) => Platform::ViveUnknown,
            ("YVR", _, _, _) => Platform::Yvr,
            ("Play For Dream", _, _, _) => Platform::PlayForDreamMR,
            ("Lynx Mixed Reality", _, _, _) => Platform::Lynx,
            ("samsung", _, "xrvst2", _) => Platform::SamsungGalaxyXR,
            _ => Platform::AndroidUnknown,
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        match std::env::consts::OS {
            "visionos" => Platform::VisionOSHeadset,
            "windows" => Platform::WindowsPc,
            "linux" => Platform::LinuxPc,
            "macos" => Platform::Macos,
            _ => Platform::Unknown,
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn local_ip() -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    local_ip_address::local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum ClientFlavor {
    Store,
    Github,
    Custom(String),
}
