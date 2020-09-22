use serde::{Deserialize, Serialize};
use settings_schema::*;

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum StreamMode {
    PreferReliable,
    PreferSequentialUnreliable,
    PreferUnorderedUnreliable,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuicConfig {
    // quinn::ClientConfig / ServerConfig
    pub enable_0rtt: bool,
    pub enable_keylog: bool,
    pub use_stateless_retry: Option<bool>,

    // quinn::TransportConfig
    pub stream_window_bidi: Option<u64>,
    pub stream_window_uni: Option<u64>,
    pub max_idle_timeout_ms: Option<Switch<u64>>,
    pub stream_receive_window: Option<u64>,
    pub receive_window: Option<u64>,
    pub send_window: Option<u64>,
    pub max_tlps: Option<u32>,
    pub packet_threshold: Option<u32>,
    pub time_threshold: Option<f32>,
    pub initial_rtt_ms: Option<u64>,
    pub persistent_congestion_threshold: Option<u32>,
    pub keep_alive_interval_ms: Option<Switch<u64>>,
    pub crypto_buffer_size: Option<u64>,
    pub allow_spin: Option<bool>,
    pub datagram_receive_buffer_size: Option<Switch<u64>>,
    pub datagram_send_buffer_size: Option<u64>,
}

#[allow(clippy::large_enum_variant)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum SocketConfig {
    Udp,

    Tcp,

    #[schema(advanced)]
    Quic(QuicConfig),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

#[derive(SettingsSchema, Serialize, Deserialize, PartialEq, Clone, Default)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FoveatedRenderingDesc {
    #[schema(min = 0.5, max = 10., step = 0.1)]
    pub strength: f32,

    #[schema(advanced, min = 0.5, max = 2., step = 0.1)]
    pub shape: f32,

    #[schema(min = -0.05, max = 0.05, step = 0.001)]
    pub vertical_offset: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Debug, Clone)]
#[repr(u8)]
#[serde(rename_all = "camelCase")]
pub enum CodecType {
    H264,
    Hevc,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoDesc {
    #[schema(advanced)]
    pub adapter_index: i32,

    #[schema(advanced)]
    pub fps: Option<u32>,

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
    pub left_eye_fov: Option<Fov>,

    #[schema(advanced)]
    pub seconds_from_vsync_to_photons: f32,

    #[schema(advanced)]
    pub ipd: f32,

    pub foveated_rendering: Switch<FoveatedRenderingDesc>,
    pub color_correction: Switch<ColorCorrectionDesc>,

    pub codec: CodecType,

    #[schema(min = 1, max = 250)]
    pub encode_bitrate_mbs: u64,

    #[schema(advanced, min = 5, max = 1000)]
    pub keyframe_resend_interval_ms: u64,

    pub stream_mode: StreamMode,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioDesc {
    // deviceDropdown should poll the available audio devices and set "device"
    #[schema(placeholder = "device_dropdown")]
    //
    #[schema(advanced)]
    pub device: String,

    pub stream_mode: StreamMode,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioSection {
    pub game_audio: Switch<AudioDesc>,
    pub microphone: bool,
    pub microphone_stream_mode: StreamMode,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

    pub pose_time_offset: f32,

    #[schema(advanced)]
    pub position_offset_left: [f32; 3],

    #[schema(advanced)]
    pub rotation_offset_left: [f32; 3],

    #[schema(min = 0., max = 5., step = 0.1)]
    pub haptics_intensity: f32,

    pub haptics_stream_mode: StreamMode,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

    #[schema(advanced)]
    pub use_tracking_reference: bool,

    pub force_3dof: bool,

    pub tracking_stream_mode: StreamMode,

    pub controllers: Switch<ControllersDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDesc {
    pub stream_socket_config: SocketConfig,

    #[schema(advanced, min = 1024, max = 65535)]
    pub stream_port: u16,

    #[schema(advanced)]
    pub web_server_port: u16,

    // clientRecvBufferSize=max(encodeBitrateMbs * 2 + bufferOffset, 0)
    #[schema(placeholder = "buffer_offset")]
    //
    #[schema(advanced)]
    pub client_recv_buffer_size: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraDesc {
    pub revert_confirm_dialog: bool,
    pub restart_confirm_dialog: bool,

    #[schema(advanced)]
    pub notification_level: LogLevel,
    #[schema(advanced)]
    pub exclude_notifications_without_id: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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
            fps: OptionalDefault {
                set: false,
                content: 72,
            },
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
            left_eye_fov: OptionalDefault {
                set: false,
                content: FovDefault {
                    left: 52.,
                    right: 42.,
                    top: 53.,
                    bottom: 47.,
                },
            },
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
                enabled: true,
                content: ColorCorrectionDescDefault {
                    brightness: 0.,
                    contrast: 0.,
                    saturation: 0.,
                    gamma: 1.12,
                    sharpening: 0.,
                },
            },
            codec: CodecTypeDefault {
                variant: CodecTypeDefaultVariant::H264,
            },
            encode_bitrate_mbs: 30,
            keyframe_resend_interval_ms: 100,
            stream_mode: StreamModeDefault {
                variant: StreamModeDefaultVariant::PreferReliable,
            },
        },
        audio: AudioSectionDefault {
            game_audio: SwitchDefault {
                enabled: true,
                content: AudioDescDefault {
                    device: "".into(),
                    stream_mode: StreamModeDefault {
                        variant: StreamModeDefaultVariant::PreferSequentialUnreliable,
                    },
                },
            },
            microphone: false,
            microphone_stream_mode: StreamModeDefault {
                variant: StreamModeDefaultVariant::PreferSequentialUnreliable,
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
            use_tracking_reference: false,
            force_3dof: false,
            tracking_stream_mode: StreamModeDefault {
                variant: StreamModeDefaultVariant::PreferUnorderedUnreliable,
            },
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
                    haptics_stream_mode: StreamModeDefault {
                        variant: StreamModeDefaultVariant::PreferUnorderedUnreliable,
                    },
                },
            },
        },
        connection: ConnectionDescDefault {
            stream_socket_config: SocketConfigDefault {
                variant: SocketConfigDefaultVariant::Tcp,
                Quic: {
                    const EXPECTED_RTT: u64 = 100;
                    const MAX_STREAM_BANDWIDTH: u64 = 12500 * 1000;
                    const STREAM_RWND: u64 = MAX_STREAM_BANDWIDTH / 1000 * EXPECTED_RTT;
                    QuicConfigDefault {
                        enable_0rtt: false,
                        enable_keylog: false,
                        use_stateless_retry: OptionalDefault {
                            set: true,
                            content: true,
                        },
                        stream_window_bidi: OptionalDefault {
                            set: false,
                            content: 32,
                        },
                        stream_window_uni: OptionalDefault {
                            set: false,
                            content: 32,
                        },
                        max_idle_timeout_ms: OptionalDefault {
                            set: false,
                            content: SwitchDefault {
                                enabled: true,
                                content: 10_000,
                            },
                        },
                        stream_receive_window: OptionalDefault {
                            set: false,
                            content: STREAM_RWND,
                        },
                        receive_window: OptionalDefault {
                            set: false,
                            content: 8 * STREAM_RWND,
                        },
                        send_window: OptionalDefault {
                            set: false,
                            content: 8 * STREAM_RWND,
                        },
                        max_tlps: OptionalDefault {
                            set: false,
                            content: 2,
                        },
                        packet_threshold: OptionalDefault {
                            set: false,
                            content: 3,
                        },
                        time_threshold: OptionalDefault {
                            set: false,
                            content: 9.0 / 8.0,
                        },
                        initial_rtt_ms: OptionalDefault {
                            set: false,
                            content: 333,
                        },
                        persistent_congestion_threshold: OptionalDefault {
                            set: false,
                            content: 3,
                        },
                        keep_alive_interval_ms: OptionalDefault {
                            set: false,
                            content: SwitchDefault {
                                enabled: false,
                                content: 0,
                            },
                        },
                        crypto_buffer_size: OptionalDefault {
                            set: false,
                            content: 16 * 1024,
                        },
                        allow_spin: OptionalDefault {
                            set: false,
                            content: true,
                        },
                        datagram_receive_buffer_size: OptionalDefault {
                            set: false,
                            content: SwitchDefault {
                                enabled: true,
                                content: STREAM_RWND,
                            },
                        },
                        datagram_send_buffer_size: OptionalDefault {
                            set: false,
                            content: 1024 * 1024,
                        },
                    }
                },
            },
            stream_port: 9944,
            web_server_port: 8082,
            client_recv_buffer_size: 60_000,
        },
        extra: ExtraDescDefault {
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
