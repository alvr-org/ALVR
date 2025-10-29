#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::*;

use alvr_common::settings_schema::SettingsSchema;
use serde::{Deserialize, Serialize};
use std::env::consts;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

pub const PACKAGE_NAME_STORE: &str = "alvr.client";
pub const PACKAGE_NAME_GITHUB_DEV: &str = "alvr.client.dev";
pub const PACKAGE_NAME_GITHUB_STABLE: &str = "alvr.client.stable";

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PlatformType {
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
    AndroidUnknown,
    VisionOSHeadset,
    WindowsPc,
    LinuxPc,
    Macos,
    Unknown,
}

impl Display for PlatformType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PlatformType::Quest1 => "Quest 1",
            PlatformType::Quest2 => "Quest 2",
            PlatformType::Quest3 => "Quest 3",
            PlatformType::Quest3S => "Quest 3S",
            PlatformType::QuestPro => "Quest Pro",
            PlatformType::QuestUnknown => "Quest (unknown)",
            PlatformType::PicoNeo3 => "Pico Neo 3",
            PlatformType::Pico4 => "Pico 4",
            PlatformType::Pico4Pro => "Pico 4 Pro",
            PlatformType::Pico4Enterprise => "Pico 4 Enterprise",
            PlatformType::Pico4Ultra => "Pico 4 Ultra",
            PlatformType::PicoG3 => "Pico G3",
            PlatformType::PicoUnknown => "Pico (unknown)",
            PlatformType::Focus3 => "VIVE Focus 3",
            PlatformType::FocusVision => "VIVE Focus Vision",
            PlatformType::XRElite => "VIVE XR Elite",
            PlatformType::ViveUnknown => "HTC VIVE (unknown)",
            PlatformType::Yvr => "YVR",
            PlatformType::PlayForDreamMR => "Play For Dream MR",
            PlatformType::Lynx => "Lynx Headset",
            PlatformType::AndroidUnknown => "Android (unknown)",
            PlatformType::VisionOSHeadset => "VisionOS Headset",
            PlatformType::WindowsPc => "Windows PC",
            PlatformType::LinuxPc => "Linux PC",
            PlatformType::Macos => "macOS",
            PlatformType::Unknown => "Unknown",
        };
        write!(f, "{name}")
    }
}

// Platform of the device. It is used to match the VR runtime and enable features conditionally.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Platform {
    platform_type: PlatformType,
    os: String,
    manufacturer: Option<String>,
    model: Option<String>,
    device: Option<String>,
    product: Option<String>,
    runtime_name: Option<String>,
    runtime_version: Option<u64>,
}

impl Deref for Platform {
    type Target = PlatformType;

    fn deref(&self) -> &PlatformType {
        &self.platform_type
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Platform: {}, OS: {}, Manufacturer: {}, Model: {}, \
            Device: {}, Product: {}, Runtime Name: {}, Runtime Version: {}",
            self.platform_type,
            self.os,
            self.manufacturer.as_deref().unwrap_or("Unknown"),
            self.model.as_deref().unwrap_or("Unknown"),
            self.device.as_deref().unwrap_or("Unknown"),
            self.product.as_deref().unwrap_or("Unknown"),
            self.runtime_name.as_deref().unwrap_or("Unknown"),
            self.runtime_version
                .map_or("Unknown".to_string(), |v| v.to_string())
        )
    }
}

impl Platform {
    pub fn platform_type(&self) -> PlatformType {
        self.platform_type
    }

    pub fn is_quest(&self) -> bool {
        self.manufacturer.as_deref() == Some("Oculus")
    }

    pub fn is_pico(&self) -> bool {
        self.manufacturer.as_deref() == Some("Pico")
    }

    pub fn is_vive(&self) -> bool {
        self.manufacturer.as_deref() == Some("HTC")
    }

    pub fn is_yvr(&self) -> bool {
        self.manufacturer
            .as_deref()
            .is_some_and(|m| m == "YVR" || m == "Play For Dream")
    }

    pub fn os(&self) -> &str {
        &self.os
    }

    pub fn manufacturer(&self) -> Option<&str> {
        self.manufacturer.as_deref()
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    pub fn product(&self) -> Option<&str> {
        self.product.as_deref()
    }

    pub fn runtime_name(&self) -> Option<&str> {
        self.runtime_name.as_deref()
    }

    pub fn runtime_version(&self) -> Option<u64> {
        self.runtime_version
    }
}

pub fn platform(runtime_name: Option<String>, runtime_version: Option<u64>) -> Platform {
    #[cfg(target_os = "android")]
    {
        let manufacturer = android::manufacturer_name();
        let model = android::model_name();
        let device = android::device_name();
        let product = android::product_name();

        let platform_type = match (
            manufacturer.as_str(),
            model.as_str(),
            device.as_str(),
            product.as_str(),
        ) {
            ("Oculus", _, "monterey", _) => PlatformType::Quest1,
            ("Oculus", _, "hollywood", _) => PlatformType::Quest2,
            ("Oculus", _, "eureka", _) => PlatformType::Quest3,
            ("Oculus", _, "panther", _) => PlatformType::Quest3S,
            ("Oculus", _, "seacliff", _) => PlatformType::QuestPro,
            ("Oculus", _, _, _) => PlatformType::QuestUnknown,
            ("Pico", "Pico Neo 3" | "Pico Neo3 Link", _, _) => PlatformType::PicoNeo3,
            ("Pico", _, _, "PICO 4 Pro") => PlatformType::Pico4Pro,
            ("Pico", _, _, "PICO 4 Enterprise") => PlatformType::Pico4Enterprise,
            ("Pico", _, _, "PICO 4") => PlatformType::Pico4,
            ("Pico", _, _, "PICO 4 Ultra") => PlatformType::Pico4Ultra,
            ("Pico", _, _, "PICO G3") => PlatformType::PicoG3,
            ("Pico", _, _, _) => PlatformType::PicoUnknown,
            ("HTC", "VIVE Focus 3", _, _) => PlatformType::Focus3,
            ("HTC", "VIVE Focus Vision", _, _) => PlatformType::FocusVision,
            ("HTC", "VIVE XR Series", _, _) => PlatformType::XRElite,
            ("HTC", _, _, _) => PlatformType::ViveUnknown,
            ("YVR", _, _, _) => PlatformType::Yvr,
            ("Play For Dream", _, _, _) => PlatformType::PlayForDreamMR,
            ("Lynx Mixed Reality", _, _, _) => PlatformType::Lynx,
            _ => PlatformType::AndroidUnknown,
        };

        Platform {
            platform_type,
            os: consts::OS.to_owned(),
            manufacturer: Some(manufacturer),
            model: Some(model),
            device: Some(device),
            product: Some(product),
            runtime_name,
            runtime_version,
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let platform_type = match consts::OS {
            "visionos" => PlatformType::VisionOSHeadset,
            "windows" => PlatformType::WindowsPc,
            "linux" => PlatformType::LinuxPc,
            "macos" => PlatformType::Macos,
            _ => PlatformType::Unknown,
        };

        Platform {
            platform_type,
            os: consts::OS.to_owned(),
            manufacturer: None,
            model: None,
            device: None,
            product: None,
            runtime_name,
            runtime_version,
        }
    }
}

pub fn openxr_loader_fname() -> String {
    let provisional_platform = platform(None, None);

    let loader_suffix = match provisional_platform.platform_type() {
        PlatformType::Quest1 => "_quest1",
        PlatformType::PicoNeo3
        | PlatformType::PicoG3
        | PlatformType::Pico4
        | PlatformType::Pico4Pro
        | PlatformType::Pico4Enterprise => "_pico_old",
        _ if provisional_platform.is_yvr() => "_yvr",
        PlatformType::Lynx => "_lynx",
        _ => "",
    };

    format!("libopenxr_loader{loader_suffix}.so")
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
