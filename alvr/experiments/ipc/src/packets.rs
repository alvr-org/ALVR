use alvr_common::{Fov, OpenvrPropValue};
use nalgebra::{UnitQuaternion, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackedDeviceType {
    Hmd,
    LeftHand,
    RightHand,
    GenericTracker,
}

// This packet groups all data that is updated rarely
#[derive(Serialize, Deserialize)]
pub struct DriverConfigUpdate {
    pub preferred_view_size: (u32, u32),
    pub fov: [Fov; 2],
    pub ipd_m: f32,
    pub fps: f32,
    pub battery: Vec<(TrackedDeviceType, f32)>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayConfig {
    pub presentation: bool,
    pub config: DriverConfigUpdate,
}

#[derive(Serialize, Deserialize)]
pub struct Layer {
    pub orientation: UnitQuaternion<f32>,
    pub fov: Fov,
    pub swaphcain_id: u64,
    pub rect_offset: (f32, f32),
    pub rect_size: (f32, f32),
}

#[derive(Serialize, Deserialize)]
pub enum DriverRequest {
    GetInitializationConfig,
    GetExtraProperties(u64), // device index
    CreateSwapchain {
        images_count: usize,
        width: u32,
        height: u32,
        format: u32, // interpreted as Directx or Vulkan
        sample_count: u32,
    },
    DestroySwapchain {
        id: u64,
    },
    GetNextSwapchainIndex {
        id: u64,
    },
    PresentLayers(Vec<Vec<Layer>>),
    Haptics {
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ButtonType {
    Boolean,
    Scalar,
}

#[derive(Serialize, Deserialize)]
pub enum ButtonValue {
    Boolean(bool),
    Scalar(f32),
}

#[derive(Serialize, Deserialize)]
pub struct TrackedDeviceConfig {
    pub serial_number: String,
    pub device_type: TrackedDeviceType,
    pub available_buttons: Vec<(String, ButtonType)>,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseForDriver {
    Ok,
    InitializationConfig {
        tracked_devices: Vec<TrackedDeviceConfig>,
        display_config: Option<DisplayConfig>, // None if there is no Hmd tracked device
    },
    ExtraProperties(Vec<(String, OpenvrPropValue)>),
    Swapchain {
        id: u64,
        textures: Vec<u64>, // HANDLEs or file descriptors
    },
    SwapchainIndex(usize),
}

#[derive(Serialize, Deserialize)]
pub struct MotionData {
    pub position: Vector3<f32>,
    pub orientation: UnitQuaternion<f32>,
    pub linear_velocity: Option<Vector3<f32>>,
    pub angular_velocity: Option<Vector3<f32>>,
}

#[derive(Serialize, Deserialize)]
pub enum SsePacket {
    UpdateConfig(DriverConfigUpdate),
    PropertyChanged {
        name: String,
        value: OpenvrPropValue,
    },
    TrackingData {
        trackers_data: Vec<Option<MotionData>>,
        hand_skeleton_motions: [Option<Box<[MotionData; 25]>>; 2],
        target_time_offset: Duration, // controls black pull and controller jitter
    },
    ButtonsData(Vec<(String, ButtonValue)>),
    Restart,
}
