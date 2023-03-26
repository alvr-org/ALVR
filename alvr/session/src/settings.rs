use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use settings_schema::{
    DictionaryDefault, EntryData, SettingsSchema, Switch, SwitchDefault, VectorDefault,
};

include!(concat!(env!("OUT_DIR"), "/openvr_property_keys.rs"));

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

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum EncoderQualityPreset {
    Quality = 0,
    Balanced = 1,
    Speed = 2,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum NvencTuningPreset {
    HighQuality = 1,
    LowLatency = 2,
    UltraLowLatency = 3,
    Lossless = 4,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum NvencMultiPass {
    Disabled = 0,
    QuarterResolution = 1,
    FullResolution = 2,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum NvencAdaptiveQuantizationMode {
    Disabled = 0,
    Spatial = 1,
    Temporal = 2,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "content")]
pub enum RateControlMode {
    CBR = 0,
    VBR = 1,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "content")]
pub enum EntropyCoding {
    CABAC = 0,
    CAVLC = 1,
}

/// Except for preset, the value of these fields is not applied if == -1 (flag)
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NvencOverrides {
    pub tuning_preset: NvencTuningPreset,
    pub multi_pass: NvencMultiPass,
    pub adaptive_quantization_mode: NvencAdaptiveQuantizationMode,
    pub low_delay_key_frame_scale: i64,
    pub refresh_rate: i64,
    pub enable_intra_refresh: bool,
    pub intra_refresh_period: i64,
    pub intra_refresh_count: i64,
    pub max_num_ref_frames: i64,
    pub gop_length: i64,
    pub p_frame_strategy: i64,
    pub rate_control_mode: i64,
    pub rc_buffer_size: i64,
    pub rc_initial_delay: i64,
    pub rc_max_bitrate: i64,
    pub rc_average_bitrate: i64,
    pub enable_weighted_prediction: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AmfControls {
    pub enable_vbaq: bool,
    pub use_preproc: bool,
    #[schema(min = 0, max = 10)]
    pub preproc_sigma: u32,
    #[schema(min = 0, max = 10)]
    pub preproc_tor: u32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum MediacodecDataType {
    Float(f32),
    Int32(i32),
    Int64(i64),
    String(String),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedCodecOptions {
    pub encoder_quality_preset: EncoderQualityPreset,
    pub nvenc_overrides: NvencOverrides,
    pub amf_controls: AmfControls,
    pub mediacodec_extra_options: Vec<(String, MediacodecDataType)>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum BitrateMode {
    #[schema(min = 1, max = 1000)]
    ConstantMbps(u64),
    #[serde(rename_all = "camelCase")]
    Adaptive {
        #[schema(min = 0.5, max = 2.0, step = 0.05)]
        saturation_multiplier: f32,

        #[schema(min = 1, max = 1000, step = 1)]
        max_bitrate_mbps: Switch<u64>,

        #[schema(min = 1, max = 1000, step = 1)]
        min_bitrate_mbps: Switch<u64>,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BitrateConfig {
    pub mode: BitrateMode,

    #[schema(advanced, min = 0.01, max = 2.0, step = 0.01)]
    pub framerate_reset_threshold_multiplier: f32,

    #[schema(advanced, min = 1, max = 50, step = 1)]
    pub max_network_latency_ms: Switch<u64>,

    #[schema(advanced, min = 1, max = 50)]
    pub max_decoder_latency_ms: u64,

    #[schema(advanced, min = 1, max = 100)]
    pub decoder_latency_overstep_frames: u64,

    #[schema(advanced, min = 0.5, max = 1.0)]
    pub decoder_latency_overstep_multiplier: f32,
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

    #[schema(advanced, min = 1., max = 10.0, step = 0.1)]
    pub max_buffering_frames: f32,

    #[schema(advanced, min = 0.50, max = 0.99, step = 0.01)]
    pub buffering_history_weight: f32,

    pub codec: CodecType,

    #[schema(advanced)]
    pub rate_control_mode: RateControlMode,

    #[schema(advanced)]
    pub filler_data: bool,

    #[schema(advanced)]
    pub entropy_coding: EntropyCoding,

    pub use_10bit_encoder: bool,

    #[schema(advanced)]
    pub force_sw_encoding: bool,

    #[schema(advanced)]
    pub sw_thread_count: u32,

    pub bitrate: BitrateConfig,

    #[schema(advanced)]
    pub advanced_codec_options: AdvancedCodecOptions,

    pub foveated_rendering: Switch<FoveatedRenderingDesc>,
    pub oculus_foveation_level: OculusFovetionLevel,
    pub dynamic_oculus_foveation: bool,
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioBufferingConfig {
    #[schema(min = 0, max = 200)]
    pub average_buffering_ms: u64,

    #[schema(advanced, min = 1, max = 20)]
    pub batch_ms: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioSection {
    #[schema(advanced)]
    pub linux_backend: LinuxAudioBackend,

    pub game_audio: Switch<GameAudioDesc>,

    pub microphone: Switch<MicrophoneDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
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

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenvrPropEntry {
    pub key: OpenvrPropertyKey,
    pub value: OpenvrPropValue,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum HeadsetEmulationMode {
    RiftS,
    Vive,
    Quest2,
    #[serde(rename_all = "camelCase")]
    Custom {
        serial_number: String,
        props: Vec<OpenvrPropEntry>,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum ControllersEmulationMode {
    RiftSTouch,
    ValveIndex,
    ViveWand,
    Quest2Touch,
    ViveTracker,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControllersTriggerOverrideDesc {
    #[schema(advanced, min = 0.01, max = 1., step = 0.01)]
    pub trigger_threshold: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControllersGripOverrideDesc {
    #[schema(advanced, min = 0.01, max = 1., step = 0.01)]
    pub grip_threshold: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct HapticsConfig {
    #[schema(min = 0., max = 5., step = 0.1)]
    pub intensity_multiplier: f32,

    #[schema(advanced, min = 0., max = 1., step = 0.01)]
    pub amplitude_curve: f32,

    #[schema(advanced, min = 0., max = 0.1, step = 0.001)]
    pub min_duration_s: f32,

    #[schema(advanced, min = 1., max = 5., step = 0.1)]
    pub low_duration_amplitude_multiplier: f32,

    #[schema(advanced, min = 0., max = 1., step = 0.01)]
    pub low_duration_range_multiplier: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControllersDesc {
    pub emulation_mode: ControllersEmulationMode,

    pub extra_openvr_props: Vec<OpenvrPropEntry>,

    #[schema(min = -50, max = 50, step = 1)]
    pub pose_time_offset_ms: i64,

    #[schema(advanced, min = 0., max = 1.0, step = 0.001)]
    pub linear_velocity_cutoff: f32,

    #[schema(advanced, min = 0., max = 100., step = 1.)]
    pub angular_velocity_cutoff: f32,

    #[schema(advanced)]
    pub left_controller_position_offset: [f32; 3],

    #[schema(advanced)]
    pub left_controller_rotation_offset: [f32; 3],

    #[schema(advanced)]
    pub left_hand_tracking_position_offset: [f32; 3],

    #[schema(advanced)]
    pub left_hand_tracking_rotation_offset: [f32; 3],

    #[schema(advanced)]
    pub override_trigger_threshold: Switch<ControllersTriggerOverrideDesc>,

    #[schema(advanced)]
    pub override_grip_threshold: Switch<ControllersGripOverrideDesc>,

    pub haptics: Switch<HapticsConfig>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum PositionRecenteringMode {
    Disabled,
    LocalFloor,
    #[serde(rename_all = "camelCase")]
    Local {
        view_height: f32,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum RotationRecenteringMode {
    Disabled,
    Yaw,
    Tilted,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeadsetDesc {
    pub emulation_mode: HeadsetEmulationMode,

    pub extra_openvr_props: Vec<OpenvrPropEntry>,

    #[schema(advanced)]
    pub tracking_ref_only: bool,

    #[schema(advanced)]
    pub enable_vive_tracker_proxy: bool,

    pub controllers: Switch<ControllersDesc>,

    pub position_recentering_mode: PositionRecenteringMode,

    pub rotation_recentering_mode: RotationRecenteringMode,

    #[schema(advanced)]
    pub extra_latency_mode: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum SocketProtocol {
    Udp,
    Tcp,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryConfig {
    #[schema(advanced)]
    pub auto_trust_clients: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum SocketBufferSize {
    Default,
    Maximum,
    Custom(u32),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectionCriteria {
    pub latency_threshold_ms: u64,
    pub sustain_duration_s: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDesc {
    pub client_discovery: Switch<DiscoveryConfig>,

    #[schema(advanced, min = 1024, max = 0xFFFF)]
    pub web_server_port: u16,

    pub stream_protocol: SocketProtocol,

    #[schema(advanced)]
    pub server_send_buffer_bytes: SocketBufferSize,

    #[schema(advanced)]
    pub server_recv_buffer_bytes: SocketBufferSize,

    #[schema(advanced)]
    pub client_send_buffer_bytes: SocketBufferSize,

    #[schema(advanced)]
    pub client_recv_buffer_bytes: SocketBufferSize,

    #[schema(advanced)]
    pub stream_port: u16,

    #[schema(advanced)]
    pub aggressive_keyframe_resend: bool,

    #[schema(advanced)]
    pub on_connect_script: String,

    #[schema(advanced)]
    pub on_disconnect_script: String,

    // Max packet size is 64KB for TCP and 65507 bytes for UDP
    #[schema(advanced, min = 0, max = 0xFFFF)]
    pub packet_size: i32,

    #[schema(advanced)]
    pub statistics_history_size: u64,

    pub disconnection_criteria: Switch<DisconnectionCriteria>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum Theme {
    SystemDefault,
    Classic,
    Darkly,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum UpdateChannel {
    NoUpdates,
    Stable,
    Nightly,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Patches {
    pub remove_sync_popup: bool,
    pub linux_async_reprojection: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraDesc {
    pub theme: Theme,
    pub revert_confirm_dialog: bool,
    pub restart_confirm_dialog: bool,
    pub prompt_before_update: bool,
    pub update_channel: UpdateChannel,
    pub log_to_disk: bool,
    pub log_button_presses: bool,
    pub log_haptics: bool,
    pub save_video_stream: bool,

    #[schema(advanced)]
    pub notification_level: LogLevel,
    #[schema(advanced)]
    pub exclude_notifications_without_id: bool,

    pub capture_frame_dir: String,

    pub patches: Patches,
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
    let socket_buffer = SocketBufferSizeDefault {
        Custom: 100000,
        variant: SocketBufferSizeDefaultVariant::Maximum,
    };

    let default_custom_openvr_props = VectorDefault {
        element: OpenvrPropEntryDefault {
            key: OpenvrPropertyKeyDefault {
                variant: OpenvrPropertyKeyDefaultVariant::TrackingSystemName,
            },
            value: OpenvrPropValueDefault {
                Bool: false,
                Float: 0.0,
                Int32: 0,
                Uint64: 0,
                Vector3: [0.0, 0.0, 0.0],
                Double: 0.0,
                String: "".into(),
                variant: OpenvrPropValueDefaultVariant::String,
            },
        },
        content: vec![],
    };

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
            max_buffering_frames: 1.5,
            buffering_history_weight: 0.90,
            codec: CodecTypeDefault {
                variant: CodecTypeDefaultVariant::H264,
            },
            rate_control_mode: RateControlModeDefault {
                variant: RateControlModeDefaultVariant::CBR,
            },
            filler_data: false,
            entropy_coding: EntropyCodingDefault {
                variant: EntropyCodingDefaultVariant::CAVLC,
            },
            use_10bit_encoder: false,
            force_sw_encoding: false,
            sw_thread_count: 0,
            bitrate: BitrateConfigDefault {
                mode: BitrateModeDefault {
                    ConstantMbps: 30,
                    Adaptive: BitrateModeAdaptiveDefault {
                        saturation_multiplier: 0.95,
                        max_bitrate_mbps: SwitchDefault {
                            enabled: false,
                            content: 100,
                        },
                        min_bitrate_mbps: SwitchDefault {
                            enabled: false,
                            content: 5,
                        },
                    },
                    variant: BitrateModeDefaultVariant::Adaptive,
                },
                framerate_reset_threshold_multiplier: 0.30,
                max_network_latency_ms: SwitchDefault {
                    enabled: false,
                    content: 8,
                },
                max_decoder_latency_ms: 15,
                decoder_latency_overstep_frames: 15,
                decoder_latency_overstep_multiplier: 0.99,
            },
            advanced_codec_options: AdvancedCodecOptionsDefault {
                encoder_quality_preset: EncoderQualityPresetDefault {
                    variant: EncoderQualityPresetDefaultVariant::Speed,
                },
                nvenc_overrides: NvencOverridesDefault {
                    tuning_preset: NvencTuningPresetDefault {
                        variant: NvencTuningPresetDefaultVariant::LowLatency,
                    },
                    multi_pass: NvencMultiPassDefault {
                        variant: NvencMultiPassDefaultVariant::QuarterResolution,
                    },
                    adaptive_quantization_mode: NvencAdaptiveQuantizationModeDefault {
                        variant: NvencAdaptiveQuantizationModeDefaultVariant::Spatial,
                    },
                    low_delay_key_frame_scale: -1,
                    refresh_rate: -1,
                    enable_intra_refresh: false,
                    intra_refresh_period: -1,
                    intra_refresh_count: -1,
                    max_num_ref_frames: -1,
                    gop_length: -1,
                    p_frame_strategy: -1,
                    rate_control_mode: -1,
                    rc_buffer_size: -1,
                    rc_initial_delay: -1,
                    rc_max_bitrate: -1,
                    rc_average_bitrate: -1,
                    enable_weighted_prediction: false,
                },
                amf_controls: AmfControlsDefault {
                    enable_vbaq: false,
                    use_preproc: false,
                    preproc_sigma: 4,
                    preproc_tor: 7,
                },
                mediacodec_extra_options: DictionaryDefault {
                    key: "".into(),
                    value: MediacodecDataTypeDefault {
                        variant: MediacodecDataTypeDefaultVariant::String,
                        Float: 0.0,
                        Int32: 0,
                        Int64: 0,
                        String: "".into(),
                    },
                    content: vec![
                        ("operating-rate".into(), MediacodecDataType::Int32(i32::MAX)),
                        ("priority".into(), MediacodecDataType::Int32(0)),
                        // low-latency: only applicable on API level 30. Quest 1 and 2 might not be
                        // cabable, since they are on level 29.
                        ("low-latency".into(), MediacodecDataType::Int32(1)),
                        (
                            "vendor.qti-ext-dec-low-latency.enable".into(),
                            MediacodecDataType::Int32(1),
                        ),
                    ],
                },
            },
            foveated_rendering: SwitchDefault {
                enabled: true,
                content: FoveatedRenderingDescDefault {
                    center_size_x: 0.4,
                    center_size_y: 0.35,
                    center_shift_x: 0.4,
                    center_shift_y: 0.1,
                    edge_ratio_x: 4.,
                    edge_ratio_y: 5.,
                },
            },
            oculus_foveation_level: OculusFovetionLevelDefault {
                variant: OculusFovetionLevelDefaultVariant::HighTop,
            },
            dynamic_oculus_foveation: true,
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
            emulation_mode: HeadsetEmulationModeDefault {
                Custom: HeadsetEmulationModeCustomDefault {
                    serial_number: "Unknown".into(),
                    props: default_custom_openvr_props.clone(),
                },
                variant: HeadsetEmulationModeDefaultVariant::Quest2,
            },
            extra_openvr_props: default_custom_openvr_props.clone(),
            tracking_ref_only: false,
            enable_vive_tracker_proxy: false,
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    emulation_mode: ControllersEmulationModeDefault {
                        variant: ControllersEmulationModeDefaultVariant::Quest2Touch,
                    },
                    extra_openvr_props: default_custom_openvr_props,
                    pose_time_offset_ms: 20,
                    linear_velocity_cutoff: 0.05,
                    angular_velocity_cutoff: 10.0,
                    left_controller_position_offset: [0.0, 0.0, -0.11],
                    left_controller_rotation_offset: [-20.0, 0.0, 0.0],
                    left_hand_tracking_position_offset: [0.04, -0.02, -0.13],
                    left_hand_tracking_rotation_offset: [0.0, -45.0, -90.0],
                    override_trigger_threshold: SwitchDefault {
                        enabled: false,
                        content: ControllersTriggerOverrideDescDefault {
                            trigger_threshold: 0.1,
                        },
                    },
                    override_grip_threshold: SwitchDefault {
                        enabled: false,
                        content: ControllersGripOverrideDescDefault {
                            grip_threshold: 0.1,
                        },
                    },
                    haptics: SwitchDefault {
                        enabled: true,
                        content: HapticsConfigDefault {
                            intensity_multiplier: 1.,
                            amplitude_curve: 0.4,
                            min_duration_s: 0.01,
                            low_duration_amplitude_multiplier: 2.5,
                            low_duration_range_multiplier: 0.5,
                        },
                    },
                },
            },
            position_recentering_mode: PositionRecenteringModeDefault {
                Local: PositionRecenteringModeLocalDefault { view_height: 1.5 },
                variant: PositionRecenteringModeDefaultVariant::LocalFloor,
            },
            rotation_recentering_mode: RotationRecenteringModeDefault {
                variant: RotationRecenteringModeDefaultVariant::Yaw,
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
                variant: SocketProtocolDefaultVariant::Udp,
            },
            server_send_buffer_bytes: socket_buffer.clone(),
            server_recv_buffer_bytes: socket_buffer.clone(),
            client_send_buffer_bytes: socket_buffer.clone(),
            client_recv_buffer_bytes: socket_buffer,
            stream_port: 9944,
            aggressive_keyframe_resend: false,
            on_connect_script: "".into(),
            on_disconnect_script: "".into(),
            packet_size: 1400,
            statistics_history_size: 256,
            disconnection_criteria: SwitchDefault {
                enabled: false,
                content: DisconnectionCriteriaDefault {
                    latency_threshold_ms: 150,
                    sustain_duration_s: 3,
                },
            },
        },
        extra: ExtraDescDefault {
            theme: ThemeDefault {
                variant: ThemeDefaultVariant::SystemDefault,
            },
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
            log_button_presses: false,
            log_haptics: false,
            save_video_stream: false,
            notification_level: LogLevelDefault {
                variant: if cfg!(debug_assertions) {
                    LogLevelDefaultVariant::Info
                } else {
                    LogLevelDefaultVariant::Warning
                },
            },
            exclude_notifications_without_id: false,
            capture_frame_dir: if !cfg!(target_os = "linux") {
                "/tmp".into()
            } else {
                "".into()
            },
            patches: PatchesDefault {
                remove_sync_popup: false,
                linux_async_reprojection: false,
            },
        },
    }
}
