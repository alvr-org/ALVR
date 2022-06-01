use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use settings_schema::{EntryData, SettingsSchema, Switch, SwitchDefault};

include!(concat!(env!("OUT_DIR"), "/openvr_property_keys.rs"));

// Field of view in radians
#[derive(SettingsSchema, Serialize, Deserialize, PartialEq, Default, Clone, Copy)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum MediacodecDataType {
    Float(f32),
    Int32(i32),
    Int64(i64),
    String(String), // Note: Double, Rect and Size are for level 28 and not compatible with the Oculus Go
}

// #[derive(SettingsSchema, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct VideoCoding {
//     codec: CodecType,
//     mediacodec_extra_options: Vec<(String, MediacodecDataType)>,
// }

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyUseFrametimeDesc {
    #[schema(advanced, min = 10000, max = 100000, step = 1000)]
    pub latency_target_maximum: u64,

    #[schema(advanced, min = -4000, max = 8000, step = 500)]
    pub latency_target_offset: i32,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveBitrateDesc {
    #[schema(min = 10, max = 500, step = 1)]
    pub bitrate_maximum: u64,

    #[schema(advanced, min = 1000, max = 25000, step = 500)]
    pub latency_target: u64,

    #[schema(advanced)]
    pub latency_use_frametime: Switch<LatencyUseFrametimeDesc>,

    #[schema(advanced, min = 500, max = 5000, step = 100)]
    pub latency_threshold: u64,

    #[schema(advanced, min = 1, max = 10, step = 1)]
    pub bitrate_up_rate: u64,

    #[schema(advanced, min = 1, max = 10, step = 1)]
    pub bitrate_down_rate: u64,

    #[schema(advanced, min = 0., max = 1., step = 0.01)]
    pub bitrate_light_load_threshold: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
#[repr(u8)]
pub enum OculusFovetionLevel {
    None,
    Low,
    Medium,
    High,
    HighTop,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoveatedRenderingDesc {
    #[schema(min = 0., max = 1., step = 0.01)]
    pub center_size_x: f32,

    #[schema(min = 0., max = 1., step = 0.01)]
    pub center_size_y: f32,

    #[schema(min = -1., max = 1., step = 0.01)]
    pub center_shift_x: f32,

    #[schema(min = -1., max = 1., step = 0.01)]
    pub center_shift_y: f32,

    #[schema(min = 1., max = 10., step = 1.)]
    pub edge_ratio_x: f32,

    #[schema(min = 1., max = 10., step = 1.)]
    pub edge_ratio_y: f32,

    pub oculus_foveation_level: OculusFovetionLevel,
    pub dynamic_oculus_foveation: bool,
}

#[derive(SettingsSchema, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
#[repr(C)]
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
#[serde(tag = "type", content = "content")]
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

    #[schema(placeholder = "display_refresh_rate")]
    //
    #[schema(advanced)]
    pub preferred_fps: f32,

    pub codec: CodecType,

    // #[schema(advanced)]
    // pub video_coding: VideoCoding,
    #[schema(advanced)]
    pub client_request_realtime_decoder: bool,

    pub use_10bit_encoder: bool,

    #[schema(advanced)]
    pub force_sw: bool,

    #[schema(advanced)]
    pub sw_thread_count: u32,

    #[schema(min = 1, max = 500)]
    pub encode_bitrate_mbs: u64,

    pub adaptive_bitrate: Switch<AdaptiveBitrateDesc>,

    #[schema(advanced)]
    pub seconds_from_vsync_to_photons: f32,

    pub foveated_rendering: Switch<FoveatedRenderingDesc>,
    pub color_correction: Switch<ColorCorrectionDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum AudioDeviceId {
    Default,
    Name(String),
    #[schema(min = 1, gui = "UpDown")]
    Index(u64),
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBufferingConfig {
    #[schema(min = 0, max = 200)]
    pub average_buffering_ms: u64,

    #[schema(advanced, min = 1, max = 20)]
    pub batch_ms: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameAudioDesc {
    #[schema(placeholder = "device_dropdown")]
    //
    #[schema(advanced)]
    pub device_id: AudioDeviceId,
    pub mute_when_streaming: bool,
    pub buffering_config: AudioBufferingConfig,
}

// Note: sample rate is a free parameter for microphone, because both server and client supports
// resampling. In contrary, for game audio, the server does not support resampling.
#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneDesc {
    #[schema(placeholder = "input_device_dropdown")]
    //
    #[schema(advanced)]
    pub input_device_id: AudioDeviceId,

    #[schema(placeholder = "output_device_dropdown")]
    //
    #[cfg(not(target_os = "linux"))]
    #[schema(advanced)]
    pub output_device_id: AudioDeviceId,

    pub buffering_config: AudioBufferingConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum LinuxAudioBackend {
    Alsa,
    Jack,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSection {
    #[schema(advanced)]
    pub linux_backend: LinuxAudioBackend,

    pub game_audio: Switch<GameAudioDesc>,

    pub microphone: Switch<MicrophoneDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
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
    pub ctrl_type_left: String,

    #[schema(advanced)]
    pub ctrl_type_right: String,

    #[schema(advanced)]
    pub registered_device_type: String,

    #[schema(advanced)]
    pub input_profile_path: String,

    #[schema(placeholder = "tracking_speed")]
    //
    #[schema(advanced)]
    pub pose_time_offset: f32,

    #[schema(advanced)]
    pub clientside_prediction: bool,

    #[schema(advanced)]
    pub serverside_prediction: bool,

    #[schema(advanced, min = 0., max = 0.1, step = 0.001)]
    pub linear_velocity_cutoff: f32,

    #[schema(advanced, min = 0., max = 100., step = 1.)]
    pub angular_velocity_cutoff: f32,

    #[schema(advanced)]
    pub position_offset_left: [f32; 3],

    #[schema(advanced)]
    pub rotation_offset_left: [f32; 3],

    #[schema(advanced, min = 0., max = 5., step = 0.1)]
    pub haptics_intensity: f32,

    #[schema(advanced, min = 0., max = 1., step = 0.01)]
    pub haptics_amplitude_curve: f32,

    #[schema(advanced, min = 0., max = 0.1, step = 0.001)]
    pub haptics_min_duration: f32,

    #[schema(advanced, min = 1., max = 5., step = 0.1)]
    pub haptics_low_duration_amplitude_multiplier: f32,

    #[schema(advanced, min = 0., max = 1., step = 0.01)]
    pub haptics_low_duration_range: f32,

    #[schema(advanced)]
    pub use_headset_tracking_system: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patches {
    pub remove_sync_popup: bool,
    pub linux_async_reprojection: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    #[schema(advanced)]
    pub force_3dof: bool,

    #[schema(advanced)]
    pub tracking_ref_only: bool,

    #[schema(advanced)]
    pub enable_vive_tracker_proxy: bool,

    pub controllers: Switch<ControllersDesc>,

    #[schema(advanced)]
    pub extra_latency_mode: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum SocketProtocol {
    Udp,

    #[schema(advanced)]
    #[serde(rename_all = "camelCase")]
    ThrottledUdp {
        #[schema(min = 1.0, step = 0.1, gui = "UpDown")]
        bitrate_multiplier: f32,
    },

    Tcp,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryConfig {
    #[schema(advanced)]
    pub auto_trust_clients: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    #[schema(advanced)]
    pub statistics_history_size: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum Theme {
    SystemDefault,
    Classic,
    Darkly,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum UpdateChannel {
    NoUpdates,
    Stable,
    Nightly,
}

#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
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

    patches: Patches,
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
            preferred_fps: 72.,
            codec: CodecTypeDefault {
                variant: CodecTypeDefaultVariant::H264,
            },
            // video_coding: VideoCodingDefault {
            //     codec: CodecTypeDefault {
            //         variant: CodecTypeDefaultVariant::H264,
            //     },
            //     mediacodec_extra_options: DictionaryDefault {
            //         key: "".into(),
            //         value: MediacodecDataTypeDefault {
            //             variant: MediacodecDataTypeDefaultVariant::String,
            //             Float: 0.0,
            //             Int32: 0,
            //             Int64: 0,
            //             String: "".into(),
            //         },
            //         content: vec![
            //             ("operating-rate".into(), MediacodecDataType::Int32(i32::MAX)),
            //             ("priority".into(), MediacodecDataType::Int32(0)),
            //             // low-latency: only applicable on API level 30. Quest 1 and 2 might not be
            //             // cabable, since they are on level 29.
            //             ("low-latency".into(), MediacodecDataType::Int32(1)),
            //             (
            //                 "vendor.qti-ext-dec-low-latency.enable".into(),
            //                 MediacodecDataType::Int32(1),
            //             ),
            //         ],
            //     },
            // },
            client_request_realtime_decoder: true,
            use_10bit_encoder: false,
            force_sw: false,
            sw_thread_count: 0,
            encode_bitrate_mbs: 30,
            adaptive_bitrate: SwitchDefault {
                enabled: true,
                content: AdaptiveBitrateDescDefault {
                    bitrate_maximum: 200,
                    latency_target: 12000,
                    latency_use_frametime: SwitchDefault {
                        enabled: false,
                        content: LatencyUseFrametimeDescDefault {
                            latency_target_maximum: 30000,
                            latency_target_offset: 0,
                        },
                    },
                    latency_threshold: 3000,
                    bitrate_up_rate: 1,
                    bitrate_down_rate: 3,
                    bitrate_light_load_threshold: 0.7,
                },
            },
            seconds_from_vsync_to_photons: 0.005,
            foveated_rendering: SwitchDefault {
                enabled: !cfg!(target_os = "linux"),
                content: FoveatedRenderingDescDefault {
                    center_size_x: 0.4,
                    center_size_y: 0.35,
                    center_shift_x: 0.4,
                    center_shift_y: 0.1,
                    edge_ratio_x: 4.,
                    edge_ratio_y: 5.,
                    oculus_foveation_level: OculusFovetionLevelDefault {
                        variant: OculusFovetionLevelDefaultVariant::HighTop,
                    },
                    dynamic_oculus_foveation: true,
                },
            },
            color_correction: SwitchDefault {
                enabled: true,
                content: ColorCorrectionDescDefault {
                    brightness: 0.,
                    contrast: 0.,
                    saturation: 0.5,
                    gamma: 1.,
                    sharpening: 0.,
                },
            },
        },
        audio: AudioSectionDefault {
            linux_backend: LinuxAudioBackendDefault {
                variant: LinuxAudioBackendDefaultVariant::Alsa,
            },
            game_audio: SwitchDefault {
                enabled: !cfg!(target_os = "linux"),
                content: GameAudioDescDefault {
                    device_id: AudioDeviceIdDefault {
                        variant: AudioDeviceIdDefaultVariant::Default,
                        Name: "".into(),
                        Index: 1,
                    },
                    mute_when_streaming: true,
                    buffering_config: AudioBufferingConfigDefault {
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
                    #[cfg(not(target_os = "linux"))]
                    output_device_id: AudioDeviceIdDefault {
                        variant: AudioDeviceIdDefaultVariant::Default,
                        Name: "".into(),
                        Index: 1,
                    },
                    buffering_config: AudioBufferingConfigDefault {
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
            tracking_ref_only: false,
            enable_vive_tracker_proxy: false,
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
                    ctrl_type_left: "oculus_touch".into(),
                    ctrl_type_right: "oculus_touch".into(),
                    registered_device_type: "oculus/1WMGH000XX0000_Controller".into(),
                    input_profile_path: "{oculus}/input/touch_profile.json".into(),
                    pose_time_offset: 0.01,
                    clientside_prediction: false,
                    serverside_prediction: true,
                    linear_velocity_cutoff: 0.01,
                    angular_velocity_cutoff: 10.,
                    position_offset_left: [-0.0065, 0.002, -0.051],
                    rotation_offset_left: [40., 0., 0.],
                    haptics_intensity: 1.,
                    haptics_amplitude_curve: 0.4,
                    haptics_min_duration: 0.01,
                    haptics_low_duration_amplitude_multiplier: 2.5,
                    haptics_low_duration_range: 0.5,
                    use_headset_tracking_system: false,
                },
            },
            extra_latency_mode: false,
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
                variant: if !cfg!(target_os = "linux") {
                    SocketProtocolDefaultVariant::Udp
                } else {
                    SocketProtocolDefaultVariant::Tcp
                },
                ThrottledUdp: SocketProtocolThrottledUdpDefault {
                    bitrate_multiplier: 1.5,
                },
            },
            stream_port: 9944,
            aggressive_keyframe_resend: false,
            on_connect_script: "".into(),
            on_disconnect_script: "".into(),
            enable_fec: true,
            statistics_history_size: 1024,
        },
        extra: ExtraDescDefault {
            theme: ThemeDefault {
                variant: ThemeDefaultVariant::SystemDefault,
            },
            client_dark_mode: false,
            revert_confirm_dialog: true,
            restart_confirm_dialog: true,
            prompt_before_update: true,
            update_channel: UpdateChannelDefault {
                variant: if alvr_common::is_stable() && cfg!(windows) {
                    UpdateChannelDefaultVariant::Stable
                } else if alvr_common::is_nightly() && cfg!(windows) {
                    UpdateChannelDefaultVariant::Nightly
                } else {
                    UpdateChannelDefaultVariant::NoUpdates
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
            patches: PatchesDefault {
                remove_sync_popup: false,
                linux_async_reprojection: true,
            },
        },
    }
}
