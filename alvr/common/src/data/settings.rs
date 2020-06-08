use serde::{Deserialize, Serialize};
use settings_schema::*;

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameSize {
    #[schema(min = 0.25, max = 2., step = 0.25)]
    Scale(f32),

    Absolute {
        #[schema(min = 32, step = 32)]
        width: u32,
        #[schema(min = 32, step = 32)]
        height: u32,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, PartialEq)]
pub struct Fov {
    #[schema(min = -90., max = 0., step = 0.1, gui = "UpDown")]
    pub left: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub right: f32,

    #[schema(min = -90., max = 0., step = 0.1, gui = "UpDown")]
    pub top: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub bottom: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoveatedRenderingDesc {
    #[schema(min = 0.5, max = 10., step = 0.1)]
    pub strength: f32,

    #[schema(advanced, min = 0.5, max = 2., step = 0.1)]
    pub shape: f32,

    #[schema(min = -0.05, max = 0.05, step = 0.001)]
    pub vertical_offset: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct ColorCorrectionDesc {
    #[schema(min = -1., max = 1., step = 0.01)]
    pub brightness: f32,

    #[schema(min = -1., max = 1., step = 0.01)]
    pub contrast: f32,

    #[schema(min = -1., max = 1., step = 0.01)]
    pub saturation: f32,

    #[schema(min = 0., max = 5., step = 0.01)]
    pub gamma: f32,

    #[schema(min = -1., max = 5., step = 0.01)]
    pub sharpening: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub enum CodecType {
    H264,
    HEVC,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDesc {
    #[schema(advanced)]
    pub adapter_index: u32,

    #[schema(advanced)]
    pub refresh_rate: u32,

    #[schema(placeholder = "resolution_dropdown")]
    #[schema(advanced)]
    pub render_resolution: FrameSize,

    #[schema(advanced)]
    pub recommended_target_resolution: FrameSize,

    #[schema(advanced)]
    pub eye_fov: [Fov; 2],

    #[schema(advanced)]
    pub seconds_from_vsync_to_photons: f32,

    #[schema(advanced)]
    pub ipd: f32,

    pub foveated_rendering: Switch<FoveatedRenderingDesc>,
    pub color_correction: Switch<ColorCorrectionDesc>,

    pub codec: CodecType,

    #[schema(min = 1, max = 250)]
    pub encode_bitrate_mbs: u64,

    pub force_60hz: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDesc {
    #[schema(placeholder = "device_dropdown")]
    #[schema(advanced)]
    pub device: String,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSection {
    pub game_audio: Switch<AudioDesc>,
    pub microphone: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllersDesc {
    #[schema(placeholder = "controller_mode")]
    #[schema(advanced)]
    pub tracking_system_name: String,

    #[schema(advanced)]
    pub manufacturer_name: String,

    #[schema(advanced)]
    pub model_number: String,

    #[schema(advanced)]
    pub render_model_name_left: String,

    #[schema(advanced)]
    pub render_model_name_right: String,

    #[schema(advanced)]
    pub serial_number: String,

    #[schema(advanced)]
    pub ctrl_type: String,

    #[schema(advanced)]
    pub registered_device_type: String,

    #[schema(advanced)]
    pub input_profile_path: String,

    #[schema(advanced)]
    pub trigger_mode: u32,

    #[schema(advanced)]
    pub trackpad_click_mode: u32,

    #[schema(advanced)]
    pub trackpad_touch_mode: u32,

    #[schema(advanced)]
    pub back_mode: u32,

    #[schema(advanced)]
    pub recenter_button: u32,

    pub pose_time_offset: u32,

    #[schema(advanced)]
    pub position_offset_left: [f32; 3],

    #[schema(advanced)]
    pub rotation_offset_left: [f32; 3],

    #[schema(min = 0., max = 5., step = 0.1)]
    pub haptics_intensity: f32,

    #[schema(advanced)]
    pub mode_idx: i32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadsetDesc {
    #[schema(placeholder = "headset_emulation_mode")]
    #[schema(advanced)]
    pub serial_number: String,

    #[schema(advanced)]
    pub tracking_system_name: String,

    #[schema(advanced)]
    pub model_number: String,

    #[schema(advanced)]
    pub driver_version: String,

    #[schema(advanced)]
    pub manufacturer_name: String,

    #[schema(advanced)]
    pub render_model_name: String,

    #[schema(advanced)]
    pub registered_device_type: String,

    pub tracking_frame_offset: i32,

    #[schema(advanced)]
    pub position_offset: [f32; 3],

    #[schema(advanced)]
    pub use_tracking_reference: bool,

    pub force_3dof: bool,

    pub controllers: Switch<ControllersDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDesc {
    #[schema(advanced)]
    pub listen_host: String,

    #[schema(advanced)]
    pub listen_port: u16,

    #[schema(advanced)]
    pub control_host: String,

    #[schema(advanced)]
    pub control_port: u16,

    #[schema(placeholder = "disable_throttling")]
    #[schema(advanced)]
    pub throttling_bitrate_mbs: u64,

    #[schema(advanced)]
    pub sending_timeslot_us: u64,

    #[schema(advanced)]
    pub limit_timeslot_packets: u64,

    #[schema(placeholder = "buffer_offset")]
    #[schema(advanced)]
    pub client_recv_buffer_size: u64,

    #[schema(placeholder = "suppress_frame_drop")]
    #[schema(advanced)]
    pub frame_queue_size: u64,

    pub aggressive_keyframe_resend: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugDesc {
    log: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct Settings {
    pub video: VideoDesc,
    pub audio: AudioSection,
    pub headset: HeadsetDesc,
    pub connection: ConnectionDesc,

    #[schema(advanced)]
    pub debug: DebugDesc,
}

pub fn settings_cache_default() -> SettingsDefault {
    SettingsDefault {
        video: VideoDescDefault {
            adapter_index: 0,
            refresh_rate: 72,
            render_resolution: FrameSizeDefault {
                variant: FrameSizeDefaultVariant::Scale,
                Scale: 1.,
                Absolute: FrameSizeAbsoluteDefault {
                    width: 2880,
                    height: 1600,
                },
            },
            recommended_target_resolution: FrameSizeDefault {
                variant: FrameSizeDefaultVariant::Scale,
                Scale: 1.,
                Absolute: FrameSizeAbsoluteDefault {
                    width: 2880,
                    height: 1600,
                },
            },
            eye_fov: [
                FovDefault {
                    left: 45.,
                    right: 45.,
                    top: 45.,
                    bottom: 45.,
                },
                FovDefault {
                    left: 45.,
                    right: 45.,
                    top: 45.,
                    bottom: 45.,
                },
            ],
            seconds_from_vsync_to_photons: 0.005,
            ipd: 0.063,
            foveated_rendering: SwitchDefault {
                enabled: false,
                content: FoveatedRenderingDescDefault {
                    strength: 2.,
                    shape: 1.5,
                    vertical_offset: 0.,
                },
            },
            color_correction: SwitchDefault {
                enabled: false,
                content: ColorCorrectionDescDefault {
                    brightness: 0.,
                    contrast: 0.,
                    saturation: 0.,
                    gamma: 1.,
                    sharpening: 0.,
                },
            },
            codec: CodecTypeDefault {
                variant: CodecTypeDefaultVariant::H264,
            },
            encode_bitrate_mbs: 30,
            force_60hz: false,
        },
        audio: AudioSectionDefault {
            game_audio: SwitchDefault {
                enabled: true,
                content: AudioDescDefault { device: "".into() },
            },
            microphone: false,
        },
        headset: HeadsetDescDefault {
            serial_number: "1WMGH000XX0000".into(),
            tracking_system_name: "oculus".into(),
            model_number: "Oculus Rift S".into(),
            driver_version: "1.42.0".into(),
            manufacturer_name: "Oculus".into(),
            render_model_name: "generic_hmd".into(),
            registered_device_type: "oculus/1WMGH000XX0000".into(),
            tracking_frame_offset: 0,
            position_offset: [0., 0., 0.],
            use_tracking_reference: false,
            force_3dof: false,
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    tracking_system_name: "oculus".into(),
                    manufacturer_name: "Oculus".into(),
                    model_number: "Oculus Rift S".into(),
                    render_model_name_left: "oculus_rifts_controller_left".into(),
                    render_model_name_right: "oculus_rifts_controller_right".into(),
                    serial_number: "1WMGH000XX0000_Controller".into(),
                    ctrl_type: "oculus_touch".into(),
                    registered_device_type: "oculus/1WMGH000XX0000_Controller".into(),
                    input_profile_path: "{oculus}/input/touch_profile.json".into(),
                    trigger_mode: 24,
                    trackpad_click_mode: 28,
                    trackpad_touch_mode: 29,
                    back_mode: 0,
                    recenter_button: 0,
                    pose_time_offset: 0,
                    position_offset_left: [-0.007, 0.005, -0.053],
                    rotation_offset_left: [36., 0., 0.],
                    haptics_intensity: 1.,
                    mode_idx: 1,
                },
            },
        },
        connection: ConnectionDescDefault {
            listen_host: "0.0.0.0".into(),
            listen_port: 9944,
            control_host: "0.0.0.0".into(),
            control_port: 9944,
            throttling_bitrate_mbs: 0,
            sending_timeslot_us: 500,
            limit_timeslot_packets: 0,
            client_recv_buffer_size: 60_000,
            frame_queue_size: 1,
            aggressive_keyframe_resend: false,
        },
        debug: DebugDescDefault { log: true },
    }
}
