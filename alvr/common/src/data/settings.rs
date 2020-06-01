use serde::{Deserialize, Serialize};
use settings_schema::*;
use std::os::raw::c_char;

#[macro_export]
macro_rules! extern_getters {
    (@ $struct_name:ident, $field_name:ident, String) => {
        /// # Safety
        /// settings and string_buf memory must not overlap
        #[no_mangle]
        pub unsafe extern "C" fn $field_name(
            settings: &$struct_name,
            string_buf: *mut c_char,
            buf_len: usize,
        ) {
            use std::ffi::CString;

            let cstring = CString::new(settings.$field_name.clone()).unwrap();
            std::ptr::copy_nonoverlapping(
                cstring.as_ptr(),
                string_buf,
                std::cmp::min(cstring.as_bytes_with_nul().len(), buf_len),
            )
        }
    };
    (@ $struct_name:ident, $field_name:ident, [$inner_type:ty; $len:tt]) => {
        #[no_mangle]
        pub extern "C" fn $field_name<'a>(
            settings: &'a $struct_name,
            array: &mut &'a [$inner_type; $len],
        ) {
            *array = &settings.$field_name
        }
    };
    (@ $struct_name:ident, $field_name:ident, Switch<$inner_type:ty>) => {
        #[no_mangle]
        pub extern "C" fn $field_name<'a>(
            settings: &'a $struct_name,
            obj: &mut &'a $inner_type,
        ) -> bool {
            if let Switch::Enabled(o) = &settings.$field_name {
                *obj = o;
                true
            } else {
                false
            }
        }
    };
    (@ $struct_name:ident, $field_name:ident, $field_type:tt) => {
        #[no_mangle]
        pub extern "C" fn $field_name(settings: &$struct_name) -> &$field_type {
            &settings.$field_name
        }
    };

    (
        $(#[$($attrs:tt)*])*
        pub struct $struct_name:ident {
            $(
                $(#[$($field_attrs:tt)*])*
                pub $field_name:ident : $field_type:tt $(<$field_type_args:tt>)?
            ),* $(,)?
        }
    ) => {
        $(#[$($attrs)*])*
        pub struct $struct_name {
            $(
                $(#[$($field_attrs)*])*
                pub $field_name: $field_type $(<$field_type_args>)?,
            )*
        }

        $(extern_getters!(@ $struct_name, $field_name, $field_type $(<$field_type_args>)?);)*
    };
}

#[repr(C)]
#[derive(SettingsSchema, Serialize, Deserialize)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
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

#[repr(C)]
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

#[repr(C)]
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

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize)]
pub enum CodecType {
    H264,
    HEVC,
}

extern_getters! {
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
        pub nv12: bool,
    }
}

extern_getters! {
    #[derive(SettingsSchema, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AudioDesc {

        #[schema(placeholder = "device_dropdown")]

        #[schema(advanced)]
        pub device: String
    }
}

extern_getters! {
    #[derive(SettingsSchema, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AudioSection {
        pub game_audio: Switch<AudioDesc>,
        pub microphone: Switch<AudioDesc>,
    }
}

extern_getters! {
    #[derive(SettingsSchema, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ControllersDesc {
        #[schema(placeholder = "controller_mode")]

        #[schema(advanced)]
        pub ctrl_tracking_system_name: String,

        #[schema(advanced)]
        pub ctrl_manufacturer_name: String,

        #[schema(advanced)]
        pub ctrl_model_number: String,

        #[schema(advanced)]
        pub render_model_name_left: String,

        #[schema(advanced)]
        pub render_model_name_right: String,

        #[schema(advanced)]
        pub ctrl_serial_number: String,

        #[schema(advanced)]
        pub ctrl_type: String,

        #[schema(advanced)]
        pub ctrl_registered_device_type: String,

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
        pub ctrl_mode_idx: i32,
    }
}

extern_getters! {
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
}

extern_getters! {
    #[derive(SettingsSchema, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConnectionDesc {
        #[schema(advanced)]
        pub host: String,

        #[schema(advanced)]
        pub port: u16,

        #[schema(advanced)]
        pub control_host: String,

        #[schema(advanced)]
        pub control_port: u16,

        #[schema(advanced)]
        pub auto_connect_host: String,

        #[schema(advanced)]
        pub auto_connect_port: u16,

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
}

#[repr(u8)]
#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum ChoiceTest {
    A,

    B(u32),

    #[serde(rename_all = "camelCase")]
    C {
        test_c: f32,
    },
}

#[repr(C)]
#[derive(SettingsSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugDesc {
    log: bool,

    choice_test: ChoiceTest,
}

extern_getters! {
    #[derive(SettingsSchema, Serialize, Deserialize)]
    pub struct Settings {
        pub video: VideoDesc,
        pub audio: AudioSection,
        pub headset: HeadsetDesc,
        pub connection: ConnectionDesc,

        #[schema(advanced)]
        pub debug: DebugDesc,
    }
}

pub fn settings_cache_default() -> SettingsDefault {
    SettingsDefault {
        video: VideoDescDefault {
            adapter_index: 0,
            refresh_rate: 72,
            render_resolution: FrameSizeDefault {
                width: 2880,
                height: 1600,
            },
            recommended_target_resolution: FrameSizeDefault {
                width: 2880,
                height: 1600,
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
            nv12: false,
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
            use_tracking_reference: false,
            force_3dof: false,
            controllers: SwitchDefault {
                enabled: true,
                content: ControllersDescDefault {
                    ctrl_tracking_system_name: "oculus".into(),
                    ctrl_manufacturer_name: "Oculus".into(),
                    ctrl_model_number: "Oculus Rift S".into(),
                    render_model_name_left: "oculus_rifts_controller_left".into(),
                    render_model_name_right: "oculus_rifts_controller_right".into(),
                    ctrl_serial_number: "1WMGH000XX0000_Controller".into(),
                    ctrl_type: "oculus_touch".into(),
                    ctrl_registered_device_type: "oculus/1WMGH000XX0000_Controller".into(),
                    input_profile_path: "{oculus}/input/touch_profile.json".into(),
                    trigger_mode: 24,
                    trackpad_click_mode: 28,
                    trackpad_touch_mode: 29,
                    back_mode: 0,
                    recenter_button: 0,
                    pose_time_offset: 0,
                    position_offset_left: [0., 0., 0.],
                    rotation_offset_left: [36., 0., 0.],
                    haptics_intensity: 1.,
                    ctrl_mode_idx: 1,
                },
            },
        },
        connection: ConnectionDescDefault {
            host: "0.0.0.0".into(),
            port: 9944,
            control_host: "0.0.0.0".into(),
            control_port: 9944,
            auto_connect_host: "".into(),
            auto_connect_port: 0,
            throttling_bitrate_mbs: 0,
            sending_timeslot_us: 500,
            limit_timeslot_packets: 0,
            client_recv_buffer_size: 60_000,
            frame_queue_size: 1,
            aggressive_keyframe_resend: false,
        },
        debug: DebugDescDefault {
            log: true,
            choice_test: ChoiceTestDefault {
                variant: ChoiceTestDefaultVariant::B,
                B: 42,
                C: ChoiceTestCDefault { test_c: 123.456 },
            },
        },
    }
}
