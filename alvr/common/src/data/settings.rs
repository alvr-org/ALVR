use serde::{Deserialize, Serialize};
use settings_schema::*;

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum FrameSize {
    #[schema(min = 0.25, max = 2., step = 0.01)]
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
    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub left: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub right: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
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

#[derive(SettingsSchema, Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum CodecType {
    H264,
    HEVC,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDesc {
    #[schema(advanced)]
    pub adapter_index: u32,

    #[schema(placeholder = "display_refresh_rate")]
    //
    #[schema(advanced)]
    pub refresh_rate: u32,

    // Dropdown with 25%, 50%, 75%, 100%, 125%, 150% etc or custom
    // Should set renderResolution (always in scale mode).
    // When the user sets a resolution not obtainable with the preset scales, set the dropdown to
    // custom.
    // Warping compensation is already applied by the web server and driver
    #[schema(placeholder = "resolution_dropdown")]
    //
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
    // deviceDropdown should poll the available audio devices and set "device"
    #[schema(placeholder = "device_dropdown")]
    //
    #[schema(advanced)]
    pub device: String,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSection {
    pub game_audio: Switch<AudioDesc>,
    pub microphone: Switch<AudioDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllersDesc {
    // Dropdown:
    // Oculus Rift S
    // Oculus Rift S (no handtracking pinch)
    // Valve Index
    // Valve Index (no handtracking pinch)
    // modeIdx and the following strings must be set accordingly
    #[schema(placeholder = "controller_mode")]
    //
    #[schema(advanced)]
    pub mode_idx: i32,

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

    #[schema(placeholder = "tracking_speed")]
    //
    #[schema(advanced)]
    pub pose_time_offset: f32,

    #[schema(advanced)]
    pub position_offset_left: [f32; 3],

    #[schema(advanced)]
    pub rotation_offset_left: [f32; 3],

    #[schema(min = 0., max = 5., step = 0.1)]
    pub haptics_intensity: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadsetDesc {
    // Oculus Rift S or HTC Vive. Should all the following strings accordingly
    #[schema(placeholder = "headset_emulation_mode")]
    //
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

    pub force_3dof: bool,

    pub controllers: Switch<ControllersDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDesc {
    #[schema(advanced, min = 1024, max = 65535)]
    pub web_server_port: u16,

    #[schema(advanced)]
    pub listen_host: String,

    #[schema(advanced)]
    pub listen_port: u16,

    // If disableThrottling=true, set throttlingBitrateBits to 0,
    // Given audioBitrate=2000'000:
    // If false, set throttlingBitrateBits=encodeBitrateMbs * 1000'000 * 3 / 2 + audioBitrate
    #[schema(placeholder = "disable_throttling")]
    //
    #[schema(advanced)]
    pub throttling_bitrate_bits: u64,

    // clientRecvBufferSize=max(encodeBitrateMbs * 2 + bufferOffset, 0)
    #[schema(placeholder = "buffer_offset")]
    //
    #[schema(advanced)]
    pub client_recv_buffer_size: u64,

    // If suppressframeDrop=true, set frameQueueSize=5
    // If suppressframeDrop=false, set frameQueueSize=1
    #[schema(placeholder = "suppress_frame_drop")]
    //
    #[schema(advanced)]
    pub frame_queue_size: u64,

    pub aggressive_keyframe_resend: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    SystemDefault,
    Classic,
    Darkly,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraDesc {
    pub theme: Theme,
    pub revert_confirm_dialog: bool,
    pub restart_confirm_dialog: bool,

    #[schema(advanced)]
    pub notification_level: LogLevel,
    #[schema(advanced)]
    pub exclude_notifications_without_id: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct Settings {
    pub video: VideoDesc,
    pub audio: AudioSection,
    pub headset: HeadsetDesc,
    pub connection: ConnectionDesc,
    pub extra: ExtraDesc,
}

pub fn session_settings_default() -> SettingsDefault {
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
                    left: 52.,
                    right: 42.,
                    top: 53.,
                    bottom: 47.,
                },
                FovDefault {
                    left: 42.,
                    right: 52.,
                    top: 53.,
                    bottom: 47.,
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
            microphone: SwitchDefault {
                enabled: false,
                content: AudioDescDefault { device: "".into() },
            },
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
            force_3dof: false,
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    mode_idx: 1,
                    tracking_system_name: "oculus".into(),
                    manufacturer_name: "Oculus".into(),
                    model_number: "Oculus Rift S".into(),
                    render_model_name_left: "oculus_rifts_controller_left".into(),
                    render_model_name_right: "oculus_rifts_controller_right".into(),
                    serial_number: "1WMGH000XX0000_Controller".into(),
                    ctrl_type: "oculus_touch".into(),
                    registered_device_type: "oculus/1WMGH000XX0000_Controller".into(),
                    input_profile_path: "{oculus}/input/touch_profile.json".into(),
                    pose_time_offset: 0.,
                    position_offset_left: [-0.007, 0.005, -0.053],
                    rotation_offset_left: [36., 0., 0.],
                    haptics_intensity: 1.,
                },
            },
        },
        connection: ConnectionDescDefault {
            web_server_port: 8082,
            listen_host: "0.0.0.0".into(),
            listen_port: 9944,
            throttling_bitrate_bits: 30_000_000 * 3 / 2 + 2_000_000,
            client_recv_buffer_size: 60_000,
            frame_queue_size: 1,
            aggressive_keyframe_resend: false,
        },
        extra: ExtraDescDefault {
            theme: ThemeDefault {
                variant: ThemeDefaultVariant::SystemDefault,
            },
            revert_confirm_dialog: true,
            restart_confirm_dialog: true,
            notification_level: LogLevelDefault {
                variant: if cfg!(debug_assertions) {
                    LogLevelDefaultVariant::Info
                } else {
                    LogLevelDefaultVariant::Warning
                },
            },
            exclude_notifications_without_id: false,
        },
    }
}
