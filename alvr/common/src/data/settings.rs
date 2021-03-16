use serde::{Deserialize, Serialize};
use settings_schema::{DictionaryDefault, SettingsSchema, Switch, SwitchDefault, VectorDefault};

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum FrameSize {
    Scale(#[schema(min = 0.25, max = 2., step = 0.01)] f32),

    Absolute {
        #[schema(min = 32, step = 32)]
        width: u32,
        #[schema(min = 32, step = 32)]
        height: u32,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, PartialEq, Default, Clone)]
pub struct Fov {
    #[schema(min = 0., max = 90., step = 0.1, gui = "up_down")]
    pub left: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "up_down")]
    pub right: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "up_down")]
    pub top: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "up_down")]
    pub bottom: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
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

// Note: This enum cannot be converted to camelCase due to a inconsistency between generation and
// validation: "hevc" vs "hEVC".
// This is caused by serde and settings-schema using different libraries for casing conversion
// todo: don't use casing conversion also for all other structs and enums
#[derive(SettingsSchema, Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(tag = "type")]
#[schema(gui = "button_group")]
#[repr(u8)]
pub enum CodecType {
    H264,
    HEVC,
}

#[derive(SettingsSchema, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[schema(gui = "button_group")]
#[repr(u8)]
pub enum TrackingSpace {
    Local,
    Stage,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct VideoDesc {
    #[schema(advanced)]
    pub adapter_index: u32,

    #[schema(higher_order(
        name = "resolution_dropdown",
        data(choice(
            default = "75%",
            variant = "25%",
            variant = "50%",
            variant = "75%",
            variant = "100%",
            variant = "125%",
            variant = "150%",
            variant = "175%",
            variant = "200%",
        )),
        modifier = r#"{video.render_resolution.variant} = "Scale""#,
        modifier = "{video.render_resolution.content} = ({input} + 1) * 0.25",
        modifier = r#"{video.recommended_target_resolution.variant} = "Scale""#,
        modifier = "{video.recommended_target_resolution.content} = ({input} + 1) * 0.25",
    ))]
    //
    #[schema(advanced)]
    pub render_resolution: FrameSize,

    #[schema(advanced)]
    pub recommended_target_resolution: FrameSize,

    #[schema(higher_order(
        name = "display_refresh_rate",
        data(choice(
            default = "72Hz",
            gui = "button_group",
            variant = "60Hz",
            variant = "72Hz",
            variant = "80Hz",
            variant = "90Hz",
        )),
        modifier = r"
            {video.preferred_fps} = ({input} == 0) * 60 
                + ({input} == 1) * 72
                + ({input} == 2) * 80
                + ({input} == 3) * 90",
    ))]
    //
    #[schema(advanced, min = 60.0, max = 90.0)]
    pub preferred_fps: f32,

    pub codec: CodecType,

    pub use_10bit_encoder: bool,

    #[schema(advanced)]
    pub client_request_realtime_decoder: bool,

    #[schema(min = 1, max = 500)]
    pub encode_bitrate_mbs: u64,

    #[schema(advanced)]
    pub seconds_from_vsync_to_photons: f32,

    pub foveated_rendering: Switch<FoveatedRenderingDesc>,
    pub color_correction: Switch<ColorCorrectionDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "content")]
pub enum AudioDeviceId {
    Default,
    Name(String),
    Index(#[schema(min = 1, gui = "up_down")] u64),
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct AudioConfig {
    #[schema(min = 0, max = 200)]
    pub average_buffering_ms: u64,

    #[schema(advanced, min = 1, max = 20)]
    pub batch_ms: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct GameAudioDesc {
    #[schema(placeholder = "device_dropdown")]
    //
    #[schema(advanced)]
    pub device_id: AudioDeviceId,
    pub mute_when_streaming: bool,
    pub config: AudioConfig,
}

// Note: sample rate is a free parameter for microphone, because both server and client supports
// resampling. In contrary, for game audio, the server does not support resampling.
#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct MicrophoneDesc {
    #[schema(placeholder = "input_device_dropdown")]
    //
    #[schema(advanced)]
    pub input_device_id: AudioDeviceId,

    #[schema(placeholder = "output_device_dropdown")]
    //
    #[schema(advanced)]
    pub output_device_id: AudioDeviceId,

    #[schema(advanced)]
    pub sample_rate: u32,

    pub config: AudioConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct AudioSection {
    pub game_audio: Switch<GameAudioDesc>,
    pub microphone: Switch<MicrophoneDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "content")]
pub enum OpenvrPropValue {
    Bool(bool),
    Float(f32),
    Int32(i32),
    Uint64(u64),
    Vector3([f32; 3]),
    Double(f64),
    String(String),
}

#[derive(SettingsSchema, Serialize, Deserialize)]
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

    #[schema(higher_order(
        name = "tracking_speed",
        data(choice(
            default = "medium",
            gui = "button_group",
            variant = "oculus_prediction",
            variant = "slow",
            variant = "medium",
            variant = "fast",
        )),
        modifier = r"
            {headset.controllers.content.pose_time_offset} = ({input} == 1) * 0.01 
                + ({input} == 2) * -0.03
                + ({input} == 3) * -1",
        modifier = "{headset.controllers.content.clientside_prediction} = ({input} == 0)"
    ))]
    //
    #[schema(advanced)]
    pub pose_time_offset: f32,

    #[schema(advanced)]
    pub clientside_prediction: bool,

    #[schema(advanced)]
    pub position_offset_left: [f32; 3],

    #[schema(advanced)]
    pub rotation_offset_left: [f32; 3],

    #[schema(min = 0., max = 5., step = 0.1)]
    pub haptics_intensity: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct HeadsetDesc {
    #[schema(advanced)]
    pub mode_idx: u64,

    #[schema(advanced)]
    pub universe_id: u64,

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

    #[schema(advanced)]
    pub tracking_frame_offset: i32,

    #[schema(advanced)]
    pub position_offset: [f32; 3],

    pub force_3dof: bool,

    pub controllers: Switch<ControllersDesc>,

    pub tracking_space: TrackingSpace,

    #[schema(advanced)]
    pub extra_latency_mode: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
#[schema(gui = "button_group")]
pub enum SocketProtocol {
    Udp,

    ThrottledUdp {
        #[schema(advanced, min = 1.0, step = 0.1, gui = "up_down")]
        bitrate_multiplier: f32,
    },

    Tcp,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    #[schema(advanced)]
    pub auto_trust_clients: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct ConnectionDesc {
    pub client_discovery: Switch<DiscoveryConfig>,

    #[schema(advanced, min = 1024, max = 65535)]
    pub web_server_port: u16,

    pub stream_protocol: SocketProtocol,

    #[schema(advanced)]
    pub stream_port: u16,

    #[schema(advanced)]
    pub aggressive_keyframe_resend: bool,

    #[schema(advanced)]
    pub on_connect_script: String,

    #[schema(advanced)]
    pub on_disconnect_script: String,

    #[schema(advanced)]
    pub enable_fec: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
#[schema(gui = "button_group")]
pub enum Theme {
    SystemDefault,
    Classic,
    Darkly,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
#[schema(gui = "button_group")]
pub enum UpdateChannel {
    NoUpdates,
    Stable,
    Beta,
    Nightly,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct ExtraDesc {
    pub theme: Theme,
    pub client_dark_mode: bool,
    pub revert_confirm_dialog: bool,
    pub restart_confirm_dialog: bool,
    pub prompt_before_update: bool,
    pub update_channel: UpdateChannel,
    pub log_to_disk: bool,

    #[schema(advanced)]
    pub notification_level: LogLevel,
    #[schema(advanced)]
    pub exclude_notifications_without_id: bool,
    #[schema(advanced)]
    locale: String,
    #[schema(advanced)]
    show_setup_wizard: bool,

    test_vec: Vec<i32>,
    test_dict: Vec<(String, i32)>,
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
            preferred_fps: 72.,
            render_resolution: FrameSizeDefault {
                variant: FrameSizeDefaultVariant::Scale,
                Scale: 0.75,
                Absolute: FrameSizeAbsoluteDefault {
                    width: 2880,
                    height: 1600,
                },
            },
            recommended_target_resolution: FrameSizeDefault {
                variant: FrameSizeDefaultVariant::Scale,
                Scale: 0.75,
                Absolute: FrameSizeAbsoluteDefault {
                    width: 2880,
                    height: 1600,
                },
            },
            seconds_from_vsync_to_photons: 0.005,
            foveated_rendering: SwitchDefault {
                enabled: true,
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
            use_10bit_encoder: false,
            client_request_realtime_decoder: true,
            encode_bitrate_mbs: 30,
        },
        audio: AudioSectionDefault {
            game_audio: SwitchDefault {
                enabled: true,
                content: GameAudioDescDefault {
                    device_id: AudioDeviceIdDefault {
                        variant: AudioDeviceIdDefaultVariant::Default,
                        Name: "".into(),
                        Index: 1,
                    },
                    mute_when_streaming: true,
                    config: AudioConfigDefault {
                        average_buffering_ms: 50,
                        batch_ms: 10,
                    },
                },
            },
            microphone: SwitchDefault {
                enabled: false,
                content: MicrophoneDescDefault {
                    input_device_id: AudioDeviceIdDefault {
                        variant: AudioDeviceIdDefaultVariant::Default,
                        Name: "".into(),
                        Index: 1,
                    },
                    output_device_id: AudioDeviceIdDefault {
                        variant: AudioDeviceIdDefaultVariant::Default,
                        Name: "".into(),
                        Index: 1,
                    },
                    sample_rate: 44100,
                    config: AudioConfigDefault {
                        average_buffering_ms: 50,
                        batch_ms: 10,
                    },
                },
            },
        },
        headset: HeadsetDescDefault {
            mode_idx: 2,
            universe_id: 2,
            serial_number: "1WMGH000XX0000".into(),
            tracking_system_name: "oculus".into(),
            model_number: "Miramar".into(),
            driver_version: "1.55.0".into(),
            manufacturer_name: "Oculus".into(),
            render_model_name: "generic_hmd".into(),
            registered_device_type: "oculus/1WMGH000XX0000".into(),
            tracking_frame_offset: 0,
            position_offset: [0., 0., 0.],
            force_3dof: false,
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    mode_idx: 7,
                    tracking_system_name: "oculus".into(),
                    manufacturer_name: "Oculus".into(),
                    model_number: "Miramar".into(),
                    render_model_name_left: "oculus_quest2_controller_left".into(),
                    render_model_name_right: "oculus_quest2_controller_right".into(),
                    serial_number: "1WMGH000XX0000_Controller".into(),
                    ctrl_type: "oculus_touch".into(),
                    registered_device_type: "oculus/1WMGH000XX0000_Controller".into(),
                    input_profile_path: "{oculus}/input/touch_profile.json".into(),
                    pose_time_offset: 0.01,
                    clientside_prediction: false,
                    position_offset_left: [-0.007, 0.005, -0.053],
                    rotation_offset_left: [36., 0., 0.],
                    haptics_intensity: 1.,
                },
            },
            tracking_space: TrackingSpaceDefault {
                variant: TrackingSpaceDefaultVariant::Local,
            },
            extra_latency_mode: true,
        },
        connection: ConnectionDescDefault {
            client_discovery: SwitchDefault {
                enabled: true,
                content: DiscoveryConfigDefault {
                    auto_trust_clients: cfg!(debug_assertions),
                },
            },
            web_server_port: 8082,
            stream_protocol: SocketProtocolDefault {
                variant: SocketProtocolDefaultVariant::Tcp,
                ThrottledUdp: SocketProtocolThrottledUdpDefault {
                    bitrate_multiplier: 1.5,
                },
            },
            stream_port: 9944,
            aggressive_keyframe_resend: false,
            on_connect_script: "".into(),
            on_disconnect_script: "".into(),
            enable_fec: true,
        },
        extra: ExtraDescDefault {
            theme: ThemeDefault {
                variant: ThemeDefaultVariant::SystemDefault,
            },
            client_dark_mode: false,
            revert_confirm_dialog: true,
            restart_confirm_dialog: true,
            prompt_before_update: !cfg!(feature = "nightly"),
            update_channel: UpdateChannelDefault {
                variant: if cfg!(feature = "nightly") {
                    UpdateChannelDefaultVariant::Nightly
                } else {
                    UpdateChannelDefaultVariant::Stable
                },
            },
            log_to_disk: cfg!(debug_assertions),
            notification_level: LogLevelDefault {
                variant: if cfg!(debug_assertions) {
                    LogLevelDefaultVariant::Info
                } else {
                    LogLevelDefaultVariant::Warning
                },
            },
            exclude_notifications_without_id: false,
            locale: "".into(),
            show_setup_wizard: true,
            test_vec: VectorDefault {
                element: 0,
                content: vec![0],
            },
            test_dict: DictionaryDefault {
                key: "".into(),
                value: 0,
                content: vec![("".into(), 0)],
            },
        },
    }
}
