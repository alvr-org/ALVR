use alvr_common::{LogSeverity, LogSeverityDefault, LogSeverityDefaultVariant};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use settings_schema::{
    DictionaryDefault, OptionalDefault, SettingsSchema, Switch, SwitchDefault, VectorDefault,
};

include!(concat!(env!("OUT_DIR"), "/openvr_property_keys.rs"));

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum FrameSize {
    Scale(#[schema(gui(slider(min = 0.25, max = 2.0, step = 0.01)))] f32),
    Absolute {
        #[schema(gui(slider(min = 32, max = 0x1000, step = 32)))]
        width: u32,
        #[schema(gui(slider(min = 32, max = 0x1000, step = 32)))]
        height: Option<u32>,
    },
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum EncoderQualityPresetAmd {
    Quality = 0,
    Balanced = 1,
    Speed = 2,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum EncoderQualityPresetNvidia {
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
    P5 = 5,
    P6 = 6,
    P7 = 7,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum NvencTuningPreset {
    HighQuality = 1,
    LowLatency = 2,
    UltraLowLatency = 3,
    Lossless = 4,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum NvencMultiPass {
    Disabled = 0,
    #[schema(strings(display_name = "1/4 resolution"))]
    QuarterResolution = 1,
    FullResolution = 2,
}

#[repr(u32)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum NvencAdaptiveQuantizationMode {
    Disabled = 0,
    Spatial = 1,
    Temporal = 2,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum RateControlMode {
    #[schema(strings(display_name = "CBR"))]
    Cbr = 0,
    #[schema(strings(display_name = "VBR"))]
    Vbr = 1,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum EntropyCoding {
    #[schema(strings(display_name = "CAVLC"))]
    Cavlc = 1,
    #[schema(strings(display_name = "CABAC"))]
    Cabac = 0,
}

/// Except for preset, the value of these fields is not applied if == -1 (flag)
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct NvencConfig {
    #[schema(strings(
        help = "P1 is the fastest preset and P7 is the preset that produces better quality. P6 and P7 are too slow to be usable."
    ))]
    #[schema(flag = "steamvr-restart")]
    pub quality_preset: EncoderQualityPresetNvidia,
    #[schema(flag = "steamvr-restart")]
    pub tuning_preset: NvencTuningPreset,
    #[schema(strings(
        help = "Reduce compression artifacts at the cost of small performance penalty"
    ))]
    #[schema(flag = "steamvr-restart")]
    pub multi_pass: NvencMultiPass,
    #[schema(strings(
        help = r#"Spatial: Helps reduce color banding, but high-complexity scenes might look worse.
Temporal: Helps improve overall encoding quality, very small trade-off in speed."#
    ))]
    #[schema(flag = "steamvr-restart")]
    pub adaptive_quantization_mode: NvencAdaptiveQuantizationMode,
    #[schema(flag = "steamvr-restart")]
    pub low_delay_key_frame_scale: i64,
    #[schema(flag = "steamvr-restart")]
    pub refresh_rate: i64,
    #[schema(flag = "steamvr-restart")]
    pub enable_intra_refresh: bool,
    #[schema(flag = "steamvr-restart")]
    pub intra_refresh_period: i64,
    #[schema(flag = "steamvr-restart")]
    pub intra_refresh_count: i64,
    #[schema(flag = "steamvr-restart")]
    pub max_num_ref_frames: i64,
    #[schema(flag = "steamvr-restart")]
    pub gop_length: i64,
    #[schema(flag = "steamvr-restart")]
    pub p_frame_strategy: i64,
    #[schema(flag = "steamvr-restart")]
    pub rate_control_mode: i64,
    #[schema(flag = "steamvr-restart")]
    pub rc_buffer_size: i64,
    #[schema(flag = "steamvr-restart")]
    pub rc_initial_delay: i64,
    #[schema(flag = "steamvr-restart")]
    pub rc_max_bitrate: i64,
    #[schema(flag = "steamvr-restart")]
    pub rc_average_bitrate: i64,
    #[schema(flag = "steamvr-restart")]
    pub enable_weighted_prediction: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct AmfConfig {
    #[schema(flag = "steamvr-restart")]
    pub quality_preset: EncoderQualityPresetAmd,
    #[schema(strings(display_name = "Enable VBAQ"), flag = "steamvr-restart")]
    pub enable_vbaq: bool,
    #[schema(flag = "steamvr-restart")]
    pub use_preproc: bool,
    #[schema(gui(slider(min = 0, max = 10)))]
    #[schema(flag = "steamvr-restart")]
    pub preproc_sigma: u32,
    #[schema(gui(slider(min = 0, max = 10)))]
    #[schema(flag = "steamvr-restart")]
    pub preproc_tor: u32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct SoftwareEncodingConfig {
    #[schema(strings(
        display_name = "Force software encoding",
        help = "Forces the encoder to use CPU instead of GPU"
    ))]
    #[schema(flag = "steamvr-restart")]
    pub force_software_encoding: bool,

    #[schema(strings(display_name = "Encoder thread count"))]
    #[schema(flag = "steamvr-restart")]
    pub thread_count: u32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]

pub struct EncoderConfig {
    #[schema(strings(help = r#"CBR: Constant BitRate mode. This is recommended.
VBR: Variable BitRate mode. Not commended because it may throw off the adaptive bitrate algorithm. This is only supported on Windows and only with AMD/Nvidia GPUs"#))]
    #[schema(flag = "steamvr-restart")]
    pub rate_control_mode: RateControlMode,

    #[schema(strings(
        help = r#"In CBR mode, this makes sure the bitrate does not fall below the assigned value. This is mostly useful for debugging."#
    ))]
    #[schema(flag = "steamvr-restart")]
    pub filler_data: bool,

    #[schema(strings(help = r#"CAVLC algorithm is recommended.
CABAC produces better compression but it's significantly slower and may lead to runaway latency"#))]
    #[schema(flag = "steamvr-restart")]
    pub entropy_coding: EntropyCoding,

    #[schema(strings(
        display_name = "Reduce color banding",
        help = "Sets the encoder to use 10 bits per channel instead of 8. Does not work on Linux with Nvidia"
    ))]
    #[schema(flag = "steamvr-restart")]
    pub use_10bit: bool,

    #[schema(strings(display_name = "NVENC"))]
    #[schema(flag = "steamvr-restart")]
    pub nvenc: NvencConfig,

    #[schema(strings(display_name = "AMF"))]
    #[schema(flag = "steamvr-restart")]
    pub amf: AmfConfig,

    pub software: SoftwareEncodingConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
pub enum MediacodecDataType {
    Float(f32),
    Int32(i32),
    Int64(i64),
    String(String),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct DecoderLatencyFixer {
    #[schema(strings(
        display_name = "Maximum decoder latency",
        help = "When the decoder latency goes above this threshold, the bitrate will be reduced"
    ))]
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 1, max = 50)), suffix = "ms")]
    pub max_decoder_latency_ms: u64,

    #[schema(strings(
        display_name = "latency overstep",
        help = "Number of consecutive frames above the threshold to trigger a bitrate reduction"
    ))]
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 1, max = 100)), suffix = " frames")]
    pub latency_overstep_frames: u64,

    #[schema(strings(
        help = "Controls how much the bitrate is reduced when the decoder latency goes above the threshold"
    ))]
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 0.5, max = 1.0)))]
    pub latency_overstep_multiplier: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum BitrateMode {
    #[schema(strings(display_name = "Constant"))]
    ConstantMbps(#[schema(gui(slider(min = 5, max = 1000, logarithmic)), suffix = "Mbps")] u64),
    Adaptive {
        #[schema(strings(
            help = "Percentage of network bandwidth to allocate for video transmission"
        ))]
        #[schema(flag = "real-time")]
        #[schema(gui(slider(min = 0.5, max = 5.0, step = 0.01)))]
        saturation_multiplier: f32,

        #[schema(strings(display_name = "Maximum bitrate"))]
        #[schema(flag = "real-time")]
        #[schema(gui(slider(min = 1, max = 1000, logarithmic)), suffix = "Mbps")]
        max_bitrate_mbps: Switch<u64>,

        #[schema(strings(display_name = "Minimum bitrate"))]
        #[schema(flag = "real-time")]
        #[schema(gui(slider(min = 1, max = 1000, logarithmic)), suffix = "Mbps")]
        min_bitrate_mbps: Switch<u64>,

        #[schema(strings(display_name = "Maximum network latency"))]
        #[schema(flag = "real-time")]
        #[schema(gui(slider(min = 1, max = 50)), suffix = "ms")]
        max_network_latency_ms: Switch<u64>,

        #[schema(strings(
            help = "Currently there is a bug where the decoder latency keeps rising when above a certain bitrate"
        ))]
        #[schema(flag = "real-time")]
        decoder_latency_fixer: Switch<DecoderLatencyFixer>,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct BitrateAdaptiveFramerateConfig {
    #[schema(strings(
        help = "If the framerate changes more than this factor, trigger a parameters update"
    ))]
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 1.0, max = 3.0, step = 0.1)))]
    pub framerate_reset_threshold_multiplier: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct BitrateConfig {
    #[schema(flag = "real-time")]
    pub mode: BitrateMode,

    #[schema(strings(
        help = "Ensure that the specified bitrate value is respected regardless of the framerate"
    ))]
    #[schema(flag = "real-time")]
    pub adapt_to_framerate: Switch<BitrateAdaptiveFramerateConfig>,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Copy, Clone)]
pub enum ClientsideFoveationLevel {
    Low = 1,
    Medium = 2,
    High = 3,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum ClientsideFoveationMode {
    Static { level: ClientsideFoveationLevel },
    Dynamic { max_level: ClientsideFoveationLevel },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct ClientsideFoveation {
    pub mode: ClientsideFoveationMode,

    #[schema(strings(display_name = "Foveation offset"))]
    #[schema(gui(slider(min = -45.0, max = 45.0, step = 0.1)), suffix = "°")]
    pub vertical_offset_deg: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct FoveatedRenderingDesc {
    #[schema(strings(display_name = "Center region width"))]
    #[schema(gui(slider(min = 0.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub center_size_x: f32,

    #[schema(strings(display_name = "Center region height"))]
    #[schema(gui(slider(min = 0.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub center_size_y: f32,

    #[schema(strings(display_name = "Center shift X"))]
    #[schema(gui(slider(min = -1.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub center_shift_x: f32,

    #[schema(strings(display_name = "Center shift Y"))]
    #[schema(gui(slider(min = -1.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub center_shift_y: f32,

    #[schema(strings(display_name = "Horizontal edge ratio"))]
    #[schema(gui(slider(min = 1.0, max = 10.0, step = 1.0)))]
    #[schema(flag = "steamvr-restart")]
    pub edge_ratio_x: f32,

    #[schema(strings(display_name = "Vertical edge ratio"))]
    #[schema(gui(slider(min = 1.0, max = 10.0, step = 1.0)))]
    #[schema(flag = "steamvr-restart")]
    pub edge_ratio_y: f32,
}

#[repr(C)]
#[derive(SettingsSchema, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
pub struct ColorCorrectionDesc {
    #[schema(gui(slider(min = -1.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub brightness: f32,

    #[schema(gui(slider(min = -1.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub contrast: f32,

    #[schema(gui(slider(min = -1.0, max = 1.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub saturation: f32,

    #[schema(gui(slider(min = 0.0, max = 5.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub gamma: f32,

    #[schema(gui(slider(min = -1.0, max = 5.0, step = 0.01)))]
    #[schema(flag = "steamvr-restart")]
    pub sharpening: f32,
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize, Debug, Copy, Clone)]
#[schema(gui = "button_group")]
pub enum CodecType {
    #[schema(strings(display_name = "h264"))]
    H264 = 0,
    #[schema(strings(display_name = "HEVC"))]
    Hevc = 1,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct VideoDesc {
    #[schema(strings(help = "You probably don't want to change this"))]
    #[schema(flag = "steamvr-restart")]
    pub adapter_index: u32,

    #[schema(strings(
        help = "Resolution used for encoding and decoding. Relative to a single eye view."
    ))]
    #[schema(flag = "steamvr-restart")]
    pub transcoding_view_resolution: FrameSize,

    #[schema(strings(
        help = "This is the resolution that SteamVR will use as default for the game rendering. Relative to a single eye view."
    ))]
    #[schema(flag = "steamvr-restart")]
    pub emulated_headset_view_resolution: FrameSize,

    #[schema(strings(display_name = "Preferred FPS"))]
    #[schema(gui(slider(min = 60.0, max = 120.0)), suffix = "Hz")]
    #[schema(flag = "steamvr-restart")]
    pub preferred_fps: f32,

    #[schema(
        strings(
            display_name = "Maximum buffering",
            help = "Incresing this value will help reduce stutter but it will increase latency"
        ),
        gui(slider(min = 1.0, max = 10.0, step = 0.1, logarithmic)),
        suffix = " frames"
    )]
    pub max_buffering_frames: f32,

    #[schema(gui(slider(min = 0.50, max = 0.99, step = 0.01)))]
    pub buffering_history_weight: f32,

    #[schema(flag = "real-time")]
    pub bitrate: BitrateConfig,

    #[schema(strings(
        help = "HEVC may provide better visual fidelity at the cost of increased encoder latency"
    ))]
    #[schema(flag = "steamvr-restart")]
    pub preferred_codec: CodecType,

    #[schema(flag = "steamvr-restart")]
    pub encoder_config: EncoderConfig,

    pub mediacodec_extra_options: Vec<(String, MediacodecDataType)>,

    #[schema(flag = "steamvr-restart")]
    pub foveated_rendering: Switch<FoveatedRenderingDesc>,

    pub clientside_foveation: Switch<ClientsideFoveation>,

    pub dynamic_oculus_foveation: bool,

    #[schema(flag = "steamvr-restart")]
    pub color_correction: Switch<ColorCorrectionDesc>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
#[schema(gui = "button_group")]
pub enum LinuxAudioBackend {
    #[schema(strings(display_name = "ALSA"))]
    Alsa,

    Jack,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum CustomAudioDeviceConfig {
    #[schema(strings(display_name = "By name (substring)"))]
    NameSubstring(String),
    #[schema(strings(display_name = "By index"))]
    Index(usize),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct AudioBufferingConfig {
    #[schema(strings(display_name = "Average buffering"))]
    #[schema(gui(slider(min = 0, max = 200)), suffix = "ms")]
    pub average_buffering_ms: u64,

    #[schema(strings(display_name = "Batch size"))]
    #[schema(gui(slider(min = 1, max = 20)), suffix = "ms")]
    pub batch_ms: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct GameAudioConfig {
    pub device: Option<CustomAudioDeviceConfig>,
    pub mute_when_streaming: bool,
    pub buffering: AudioBufferingConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum MicrophoneDevicesConfig {
    Automatic,
    #[schema(strings(display_name = "VB Cable"))]
    VBCable,
    #[schema(strings(display_name = "VoiceMeeter"))]
    VoiceMeeter,
    #[schema(strings(display_name = "VoiceMeeter Aux"))]
    VoiceMeeterAux,
    #[schema(strings(display_name = "VoiceMeeter VAIO3"))]
    VoiceMeeterVaio3,
    Custom {
        #[schema(strings(help = "This device is used by ALVR to output microphone audio"))]
        sink: CustomAudioDeviceConfig,
        #[schema(strings(help = "This device is set in SteamVR as the default microphone"))]
        source: CustomAudioDeviceConfig,
    },
}

// Note: sample rate is a free parameter for microphone, because both server and client supports
// resampling. In contrary, for game audio, the server does not support resampling.
#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct MicrophoneConfig {
    pub devices: MicrophoneDevicesConfig,
    pub buffering: AudioBufferingConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct AudioConfig {
    #[schema(strings(help = "ALSA is recommended for most PulseAudio or PipeWire-based setups"))]
    pub linux_backend: LinuxAudioBackend,

    pub game_audio: Switch<GameAudioConfig>,

    pub microphone: Switch<MicrophoneConfig>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Debug)]
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
pub struct OpenvrPropEntry {
    pub key: OpenvrPropertyKey,
    pub value: OpenvrPropValue,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum HeadsetEmulationMode {
    #[schema(strings(display_name = "Rift S"))]
    RiftS,
    Vive,
    #[schema(strings(display_name = "Quest 2"))]
    Quest2,
    Custom {
        serial_number: String,
        props: Vec<OpenvrPropEntry>,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct FaceTrackingSources {
    pub eye_tracking_fb: bool,
    pub face_tracking_fb: bool,
    pub eye_expressions_htc: bool,
    pub lip_expressions_htc: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum FaceTrackingSinkConfig {
    #[schema(strings(display_name = "VRChat Eye OSC"))]
    VrchatEyeOsc { port: u16 },
    #[schema(strings(display_name = "VRCFaceTracking OSC"))]
    VrcFaceTrackingOsc { port: u16 },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct FaceTrackingConfig {
    pub sources: FaceTrackingSources,
    pub sink: FaceTrackingSinkConfig,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum ControllersEmulationMode {
    #[schema(strings(display_name = "Rift S Touch"))]
    RiftSTouch,
    #[schema(strings(display_name = "Valve Index"))]
    ValveIndex,
    ViveWand,
    #[schema(strings(display_name = "Quest 2 Touch"))]
    Quest2Touch,
    ViveTracker,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct HapticsConfig {
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 0.0, max = 5.0, step = 0.1)))]
    pub intensity_multiplier: f32,

    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 0.0, max = 1.0, step = 0.01)))]
    pub amplitude_curve: f32,

    #[schema(strings(display_name = "Minimum duration"))]
    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = 0.0, max = 0.1, step = 0.001)), suffix = "s")]
    pub min_duration_s: f32,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct ControllersDesc {
    #[schema(strings(
        help = "Turning this off will make the controllers appear powered off. Reconnect HMD to apply."
    ))]
    pub tracked: bool,

    #[schema(flag = "steamvr-restart")]
    pub emulation_mode: ControllersEmulationMode,

    #[schema(flag = "steamvr-restart")]
    pub extra_openvr_props: Vec<OpenvrPropEntry>,

    #[schema(strings(
        display_name = "Prediction",
        help = r"Higher values make the controllers track smoother.
Technically, this is the time (counted in frames) between pose submitted to SteamVR and the corresponding virtual vsync happens.
Currently this cannot be reliably estimated automatically. The correct value should be 2 but 3 is default for smoother tracking at the cost of slight lag."
    ))]
    #[schema(gui(slider(min = 1.0, max = 10.0, logarithmic)), suffix = "frames")]
    pub steamvr_pipeline_frames: f32,

    #[schema(flag = "real-time")]
    // note: logarithmic scale seems to be glitchy for this control
    #[schema(gui(slider(min = 0.0, max = 1.0, step = 0.01)), suffix = "m/s")]
    pub linear_velocity_cutoff: f32,

    #[schema(flag = "real-time")]
    // note: logarithmic scale seems to be glitchy for this control
    #[schema(gui(slider(min = 0.0, max = 100.0, step = 1.0)), suffix = "°/s")]
    pub angular_velocity_cutoff: f32,

    #[schema(flag = "real-time")]
    // note: logarithmic scale seems to be glitchy for this control
    #[schema(gui(slider(min = -0.5, max = 0.5, step = 0.001)), suffix = "m")]
    pub left_controller_position_offset: [f32; 3],

    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = -180.0, max = 180.0, step = 1.0)), suffix = "°")]
    pub left_controller_rotation_offset: [f32; 3],

    #[schema(flag = "real-time")]
    // note: logarithmic scale seems to be glitchy for this control
    #[schema(gui(slider(min = -0.5, max = 0.5, step = 0.001)), suffix = "m")]
    pub left_hand_tracking_position_offset: [f32; 3],

    #[schema(flag = "real-time")]
    #[schema(gui(slider(min = -180.0, max = 180.0, step = 1.0)), suffix = "°")]
    pub left_hand_tracking_rotation_offset: [f32; 3],

    #[schema(flag = "steamvr-restart")]
    #[schema(gui(slider(min = 0.01, max = 1.0, step = 0.01)))]
    pub trigger_threshold_override: Switch<f32>,

    #[schema(flag = "steamvr-restart")]
    #[schema(gui(slider(min = 0.01, max = 1.0, step = 0.01)))]
    pub grip_threshold_override: Switch<f32>,

    #[schema(flag = "real-time")]
    pub haptics: Switch<HapticsConfig>,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
pub enum PositionRecenteringMode {
    Disabled,
    LocalFloor,
    Local {
        #[schema(gui(slider(min = 0.0, max = 3.0)), suffix = "m")]
        view_height: f32,
    },
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone, Copy)]
pub enum RotationRecenteringMode {
    Disabled,
    Yaw,
    Tilted,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct HeadsetDesc {
    #[schema(flag = "steamvr-restart")]
    pub emulation_mode: HeadsetEmulationMode,

    #[schema(flag = "steamvr-restart")]
    pub extra_openvr_props: Vec<OpenvrPropEntry>,

    #[schema(flag = "steamvr-restart")]
    pub tracking_ref_only: bool,

    #[schema(flag = "steamvr-restart")]
    pub enable_vive_tracker_proxy: bool,

    pub face_tracking: Switch<FaceTrackingConfig>,

    #[schema(flag = "steamvr-restart")]
    pub controllers: Switch<ControllersDesc>,

    #[schema(strings(
        help = r#"Disabled: the playspace origin is determined by the room-scale guardian setup.
Local floor: the origin is on the floor and resets when long pressing the oculus button.
Local: the origin resets when long pressing the oculus button, and is calculated as an offset from the current head position."#
    ))]
    pub position_recentering_mode: PositionRecenteringMode,

    #[schema(strings(
        help = r#"Disabled: the playspace orientation is determined by the room-scale guardian setup.
Yaw: the forward direction is reset when long pressing the oculus button.
Tilted: the world gets tilted when long pressing the oculus button. This is useful for using VR while laying down."#
    ))]
    pub rotation_recentering_mode: RotationRecenteringMode,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
#[schema(gui = "button_group")]
pub enum SocketProtocol {
    #[schema(strings(display_name = "UDP"))]
    Udp,
    #[schema(strings(display_name = "TCP"))]
    Tcp,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct DiscoveryConfig {
    #[schema(strings(
        help = "Allow untrusted clients to connect without confirmation. This is not recommended for security reasons."
    ))]
    pub auto_trust_clients: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum SocketBufferSize {
    Default,
    Maximum,
    Custom(#[schema(suffix = "B")] u32),
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct ConnectionDesc {
    #[schema(strings(
        help = r#"UDP: Faster, but less stable than TCP. Try this if your network is well optimized and free of interference.
TCP: Slower than UDP, but more stable. Pick this if you experience video or audio stutters with UDP."#
    ))]
    pub stream_protocol: SocketProtocol,

    pub client_discovery: Switch<DiscoveryConfig>,

    pub stream_port: u16,
    pub web_server_port: u16,
    pub osc_local_port: u16,

    #[schema(strings(display_name = "Streamer send buffer size"))]
    pub server_send_buffer_bytes: SocketBufferSize,

    #[schema(strings(display_name = "Streamer receive buffer size"))]
    pub server_recv_buffer_bytes: SocketBufferSize,

    #[schema(strings(display_name = "Client send buffer size"))]
    pub client_send_buffer_bytes: SocketBufferSize,

    #[schema(strings(display_name = "Client receive buffer size"))]
    pub client_recv_buffer_bytes: SocketBufferSize,

    #[schema(strings(
        help = r#"The server discards video packets if it can't push them to the network.
This could happen on TCP. A IDR frame is requested in this case."#
    ))]
    pub max_queued_server_video_frames: usize,

    #[schema(strings(
        help = r#"If the client, server or the network discarded one packet, discard packets until a IDR packet is found.
For now works only on Windows+Nvidia"#
    ))]
    pub avoid_video_glitching: bool,

    #[schema(strings(
        help = "Reduce minimum delay between IDR keyframes from 100ms to 5ms. Use on networks with high packet loss."
    ))]
    #[schema(flag = "steamvr-restart")]
    pub aggressive_keyframe_resend: bool,

    #[schema(strings(
        help = "This script will be ran when the headset connects. Env var ACTION will be set to `connect`."
    ))]
    pub on_connect_script: String,

    #[schema(strings(
        help = "This script will be ran when the headset disconnects, or when SteamVR shuts down. Env var ACTION will be set to `disconnect`."
    ))]
    #[schema(flag = "real-time")]
    pub on_disconnect_script: String,

    #[schema(gui(slider(min = 1024, max = 65507, logarithmic)), suffix = "B")]
    pub packet_size: i32,

    #[schema(suffix = " frames")]
    pub statistics_history_size: u64,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    #[schema(strings(help = "Write logs into the session_log.txt file."))]
    pub log_to_disk: bool,
    #[schema(flag = "real-time")]
    pub log_tracking: bool,
    #[schema(flag = "real-time")]
    pub log_button_presses: bool,
    #[schema(flag = "real-time")]
    pub log_haptics: bool,
    #[schema(flag = "real-time")]
    pub notification_level: LogSeverity,
    #[schema(flag = "real-time")]
    pub show_raw_events: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum DriverLaunchAction {
    UnregisterOtherDriversAtStartup,
    #[schema(strings(display_name = "Unregister ALVR at shutdown"))]
    UnregisterAlvrAtShutdown,
    NoAction,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct SteamvrLauncher {
    #[schema(strings(
        help = r#"This controls the driver registration operations while launching SteamVR.
Unregister other drivers at startup: This is the recommended option and will handle most interferences from other installed drivers.
Unregister ALVR at shutdown: This should be used when you want to load other drivers like for full body tracking. Other VR streaming drivers like Virtual Desktop must be manually unregistered or uninstalled.
No action: All driver registration actions should be performed manually, ALVR included. This allows to launch SteamVR without launching the dashboard first."#
    ))]
    pub driver_launch_action: DriverLaunchAction,

    #[schema(strings(display_name = "Open and close SteamVR with dashboard"))]
    pub open_close_steamvr_with_dashboard: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct CaptureConfig {
    pub save_video_stream: bool,

    #[schema(flag = "steamvr-restart")]
    pub capture_frame_dir: String,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct Patches {
    #[schema(strings(help = "AMD users should keep this on. Must be off for Nvidia GPUs!",))]
    #[schema(flag = "steamvr-restart")]
    pub linux_async_reprojection: bool,
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub video: VideoDesc,
    pub audio: AudioConfig,
    pub headset: HeadsetDesc,
    pub connection: ConnectionDesc,
    pub logging: LoggingConfig,
    pub steamvr_launcher: SteamvrLauncher,
    pub capture: CaptureConfig,
    pub patches: Patches,
    pub open_setup_wizard: bool,
}

pub fn session_settings_default() -> SettingsDefault {
    let view_resolution = FrameSizeDefault {
        variant: FrameSizeDefaultVariant::Absolute,
        Scale: 1.0,
        Absolute: FrameSizeAbsoluteDefault {
            width: 2144,
            height: OptionalDefault {
                set: false,
                content: 1072,
            },
        },
    };
    let default_custom_audio_device = CustomAudioDeviceConfigDefault {
        NameSubstring: "".into(),
        Index: 0,
        variant: CustomAudioDeviceConfigDefaultVariant::NameSubstring,
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
    let socket_buffer = SocketBufferSizeDefault {
        Custom: 100000,
        variant: SocketBufferSizeDefaultVariant::Maximum,
    };

    SettingsDefault {
        video: VideoDescDefault {
            adapter_index: 0,
            transcoding_view_resolution: view_resolution.clone(),
            emulated_headset_view_resolution: view_resolution,
            preferred_fps: 72.,
            max_buffering_frames: 1.5,
            buffering_history_weight: 0.90,
            bitrate: BitrateConfigDefault {
                mode: BitrateModeDefault {
                    ConstantMbps: 30,
                    Adaptive: BitrateModeAdaptiveDefault {
                        saturation_multiplier: 1.0,
                        max_bitrate_mbps: SwitchDefault {
                            enabled: false,
                            content: 100,
                        },
                        min_bitrate_mbps: SwitchDefault {
                            enabled: false,
                            content: 5,
                        },
                        max_network_latency_ms: SwitchDefault {
                            enabled: false,
                            content: 8,
                        },
                        decoder_latency_fixer: SwitchDefault {
                            enabled: true,
                            content: DecoderLatencyFixerDefault {
                                max_decoder_latency_ms: 30,
                                latency_overstep_frames: 90,
                                latency_overstep_multiplier: 0.99,
                            },
                        },
                    },
                    variant: BitrateModeDefaultVariant::Adaptive,
                },
                adapt_to_framerate: SwitchDefault {
                    enabled: true,
                    content: BitrateAdaptiveFramerateConfigDefault {
                        framerate_reset_threshold_multiplier: 2.0,
                    },
                },
            },
            preferred_codec: CodecTypeDefault {
                variant: CodecTypeDefaultVariant::H264,
            },
            encoder_config: EncoderConfigDefault {
                rate_control_mode: RateControlModeDefault {
                    variant: RateControlModeDefaultVariant::Cbr,
                },
                filler_data: false,
                entropy_coding: EntropyCodingDefault {
                    variant: EntropyCodingDefaultVariant::Cavlc,
                },
                use_10bit: false,
                nvenc: NvencConfigDefault {
                    quality_preset: EncoderQualityPresetNvidiaDefault {
                        variant: EncoderQualityPresetNvidiaDefaultVariant::P1,
                    },
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
                amf: AmfConfigDefault {
                    quality_preset: EncoderQualityPresetAmdDefault {
                        variant: EncoderQualityPresetAmdDefaultVariant::Speed,
                    },
                    enable_vbaq: false,
                    use_preproc: false,
                    preproc_sigma: 4,
                    preproc_tor: 7,
                },
                software: SoftwareEncodingConfigDefault {
                    force_software_encoding: false,
                    thread_count: 0,
                },
            },
            mediacodec_extra_options: {
                fn int32_default(int32: i32) -> MediacodecDataTypeDefault {
                    MediacodecDataTypeDefault {
                        variant: MediacodecDataTypeDefaultVariant::Int32,
                        Float: 0.0,
                        Int32: int32,
                        Int64: 0,
                        String: "".into(),
                    }
                }
                DictionaryDefault {
                    key: "".into(),
                    value: int32_default(0),
                    content: vec![
                        ("operating-rate".into(), int32_default(i32::MAX)),
                        ("priority".into(), int32_default(0)),
                        // low-latency: only applicable on API level 30. Quest 1 and 2 might not be
                        // cabable, since they are on level 29.
                        // ("low-latency".into(), int32_default(1)), // Android smartphones crashes enabling this feature (https://github.com/PhoneVR-Developers/alvr-cardboard/issues/5)
                        (
                            "vendor.qti-ext-dec-low-latency.enable".into(),
                            int32_default(1),
                        ),
                    ],
                }
            },
            foveated_rendering: SwitchDefault {
                enabled: true,
                content: FoveatedRenderingDescDefault {
                    center_size_x: 0.45,
                    center_size_y: 0.4,
                    center_shift_x: 0.4,
                    center_shift_y: 0.1,
                    edge_ratio_x: 4.,
                    edge_ratio_y: 5.,
                },
            },
            clientside_foveation: SwitchDefault {
                enabled: true,
                content: ClientsideFoveationDefault {
                    mode: ClientsideFoveationModeDefault {
                        Static: ClientsideFoveationModeStaticDefault {
                            level: ClientsideFoveationLevelDefault {
                                variant: ClientsideFoveationLevelDefaultVariant::High,
                            },
                        },
                        Dynamic: ClientsideFoveationModeDynamicDefault {
                            max_level: ClientsideFoveationLevelDefault {
                                variant: ClientsideFoveationLevelDefaultVariant::High,
                            },
                        },
                        variant: ClientsideFoveationModeDefaultVariant::Dynamic,
                    },
                    vertical_offset_deg: 0.0,
                },
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
        audio: AudioConfigDefault {
            linux_backend: LinuxAudioBackendDefault {
                variant: LinuxAudioBackendDefaultVariant::Alsa,
            },
            game_audio: SwitchDefault {
                enabled: !cfg!(target_os = "linux"),
                content: GameAudioConfigDefault {
                    device: OptionalDefault {
                        set: false,
                        content: default_custom_audio_device.clone(),
                    },
                    mute_when_streaming: true,
                    buffering: AudioBufferingConfigDefault {
                        average_buffering_ms: 50,
                        batch_ms: 10,
                    },
                },
            },
            microphone: SwitchDefault {
                enabled: false,
                content: MicrophoneConfigDefault {
                    devices: MicrophoneDevicesConfigDefault {
                        Custom: MicrophoneDevicesConfigCustomDefault {
                            source: default_custom_audio_device.clone(),
                            sink: default_custom_audio_device,
                        },
                        variant: MicrophoneDevicesConfigDefaultVariant::Automatic,
                    },
                    buffering: AudioBufferingConfigDefault {
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
            face_tracking: SwitchDefault {
                enabled: false,
                content: FaceTrackingConfigDefault {
                    sources: FaceTrackingSourcesDefault {
                        eye_tracking_fb: true,
                        face_tracking_fb: true,
                        eye_expressions_htc: true,
                        lip_expressions_htc: true,
                    },
                    sink: FaceTrackingSinkConfigDefault {
                        VrchatEyeOsc: FaceTrackingSinkConfigVrchatEyeOscDefault { port: 9000 },
                        VrcFaceTrackingOsc: FaceTrackingSinkConfigVrcFaceTrackingOscDefault {
                            port: 9620,
                        },
                        variant: FaceTrackingSinkConfigDefaultVariant::VrchatEyeOsc,
                    },
                },
            },
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    emulation_mode: ControllersEmulationModeDefault {
                        variant: ControllersEmulationModeDefaultVariant::Quest2Touch,
                    },
                    tracked: true,
                    extra_openvr_props: default_custom_openvr_props,
                    steamvr_pipeline_frames: 3.0,
                    linear_velocity_cutoff: 0.05,
                    angular_velocity_cutoff: 10.0,
                    left_controller_position_offset: [0.0, 0.0, -0.11],
                    left_controller_rotation_offset: [-20.0, 0.0, 0.0],
                    left_hand_tracking_position_offset: [0.04, -0.02, -0.13],
                    left_hand_tracking_rotation_offset: [0.0, -45.0, -90.0],
                    trigger_threshold_override: SwitchDefault {
                        enabled: false,
                        content: 0.1,
                    },
                    grip_threshold_override: SwitchDefault {
                        enabled: false,
                        content: 0.1,
                    },
                    haptics: SwitchDefault {
                        enabled: true,
                        content: HapticsConfigDefault {
                            intensity_multiplier: 1.0,
                            amplitude_curve: 1.0,
                            min_duration_s: 0.01,
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
        },
        connection: ConnectionDescDefault {
            stream_protocol: SocketProtocolDefault {
                variant: SocketProtocolDefaultVariant::Udp,
            },
            client_discovery: SwitchDefault {
                enabled: true,
                content: DiscoveryConfigDefault {
                    auto_trust_clients: cfg!(debug_assertions),
                },
            },
            web_server_port: 8082,
            stream_port: 9944,
            osc_local_port: 9942,
            server_send_buffer_bytes: socket_buffer.clone(),
            server_recv_buffer_bytes: socket_buffer.clone(),
            client_send_buffer_bytes: socket_buffer.clone(),
            client_recv_buffer_bytes: socket_buffer,
            max_queued_server_video_frames: 1024,
            avoid_video_glitching: false,
            aggressive_keyframe_resend: false,
            on_connect_script: "".into(),
            on_disconnect_script: "".into(),
            packet_size: 1400,
            statistics_history_size: 256,
        },
        logging: LoggingConfigDefault {
            log_to_disk: cfg!(debug_assertions),
            log_button_presses: false,
            log_tracking: false,
            log_haptics: false,
            notification_level: LogSeverityDefault {
                variant: if cfg!(debug_assertions) {
                    LogSeverityDefaultVariant::Info
                } else {
                    LogSeverityDefaultVariant::Warning
                },
            },
            show_raw_events: false,
        },
        steamvr_launcher: SteamvrLauncherDefault {
            driver_launch_action: DriverLaunchActionDefault {
                variant: DriverLaunchActionDefaultVariant::UnregisterOtherDriversAtStartup,
            },
            open_close_steamvr_with_dashboard: false,
        },
        capture: CaptureConfigDefault {
            save_video_stream: false,
            capture_frame_dir: if !cfg!(target_os = "linux") {
                "/tmp".into()
            } else {
                "".into()
            },
        },
        patches: PatchesDefault {
            linux_async_reprojection: false,
        },
        open_setup_wizard: alvr_common::is_stable() || alvr_common::is_nightly(),
    }
}
