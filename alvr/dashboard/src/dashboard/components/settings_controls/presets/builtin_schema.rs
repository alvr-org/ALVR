use super::schema::{HigherOrderChoiceOption, HigherOrderChoiceSchema, PresetSchemaNode};
use settings_schema::ChoiceControlType;
use std::collections::{HashMap, HashSet};

pub fn resolution_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "resolution".into(),
        strings: [(
            "help".into(),
            "Choosing too high resolution (commonly 'High (width: 5184)') \
             may result in high latency or black screen."
                .into(),
        )]
        .into_iter()
        .collect(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            ("Very Low (width: 3072)", 1536),
            ("Low (width: 3712)", 1856),
            ("Medium (width: 4288)", 2144),
            ("High (width: 5184)", 2592),
            ("Ultra (width: 5632)", 2816),
            ("Extreme (width: 6080)", 3040),
        ]
        .into_iter()
        .map(|(key, value)| HigherOrderChoiceOption {
            name: key.into(),
            modifiers: alvr_packets::parse_path_value_pairs(&format!(
                r#"session_settings.video.transcoding_view_resolution.variant = "Absolute"
                session_settings.video.transcoding_view_resolution.Absolute.width = {value}
                session_settings.video.transcoding_view_resolution.Absolute.height.set = false
                session_settings.video.emulated_headset_view_resolution.variant = "Absolute"
                session_settings.video.emulated_headset_view_resolution.Absolute.width = {value}
                session_settings.video.emulated_headset_view_resolution.Absolute.height.set = false
                "#
            ))
            .unwrap(),
            content: None,
        })
        .collect(),
        default_option_name: "Medium (width: 4288)".into(),
        gui: ChoiceControlType::Dropdown,
    })
}

pub fn framerate_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "preferred_framerate".into(),
        strings: HashMap::new(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [60, 72, 80, 90, 120]
            .into_iter()
            .map(|framerate| HigherOrderChoiceOption {
                name: format!("{framerate}Hz"),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    "session_settings.video.preferred_fps = {}",
                    framerate as f32
                ))
                .unwrap(),
                content: None,
            })
            .collect(),
        default_option_name: "72Hz".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

pub fn codec_preset_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "codec_preset".into(),
        strings: [(
            "help".into(),
            "AV1 encoding is only supported on RDNA3, Ada Lovelace, Intel ARC or newer GPUs \
             (AMD RX 7xxx+ , NVIDIA RTX 40xx+, Intel ARC) and on headsets that have XR2 Gen 2 \
             onboard (Quest 3, Pico 4 Ultra)"
                .into(),
        )]
        .into_iter()
        .collect(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [("H264", "H264"), ("HEVC", "Hevc"), ("AV1", "AV1")]
            .into_iter()
            .map(|(key, val_codec)| HigherOrderChoiceOption {
                name: key.into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r#"session_settings.video.preferred_codec.variant = "{val_codec}""#
                ))
                .unwrap(),
                content: None,
            })
            .collect(),
        default_option_name: "H264".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

pub fn encoder_preset_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "encoder_preset".into(),
        strings: [(
            "help".into(),
            "Selecting a quality too high may result in stuttering or still image!".into(),
        )]
        .into_iter()
        .collect(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            ("Speed", "Speed", "P1"),
            ("Balanced", "Balanced", "P3"),
            ("Quality", "Quality", "P5"),
        ]
        .into_iter()
        .map(|(key, val_amd, val_nv)| HigherOrderChoiceOption {
            name: key.into(),
            modifiers: alvr_packets::parse_path_value_pairs(&format!(
                r#"session_settings.video.encoder_config.nvenc.quality_preset.variant = "{val_nv}"
                session_settings.video.encoder_config.quality_preset.variant = "{val_amd}""#
            ))
            .unwrap(),
            content: None,
        })
        .collect(),
        default_option_name: "Speed".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

pub fn foveation_preset_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "foveation_preset".into(),
        strings: [(
            "help".into(),
            "Foveation affects pixelation on the edges of \
            the screen and significantly reduces codec latency. 
It is not recommended to fully disable it, as it may cause \
shutterring and high encode/decode latency!"
                .into(),
        )]
        .into_iter()
        .collect(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            ("Light", 0.80, 0.80, 8.0, 8.0),
            ("Medium", 0.66, 0.60, 6.0, 6.0),
            ("High", 0.45, 0.40, 4.0, 5.0),
        ]
        .into_iter()
        .map(
            |(key, val_size_x, val_size_y, val_edge_x, val_edge_y)| HigherOrderChoiceOption {
                name: key.into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r"session_settings.video.foveated_encoding.enabled = true
                    session_settings.video.foveated_encoding.content.center_size_x = {val_size_x}
                    session_settings.video.foveated_encoding.content.center_size_y = {val_size_y}
                    session_settings.video.foveated_encoding.content.edge_ratio_x = {val_edge_x}
                    session_settings.video.foveated_encoding.content.edge_ratio_y = {val_edge_y}"
                ))
                .unwrap(),
                content: None,
            },
        )
        .collect(),
        default_option_name: "High".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

#[cfg(target_os = "linux")]
pub fn game_audio_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Headset speaker".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: [
            HigherOrderChoiceOption {
                name: "Disabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.audio.game_audio.enabled = false",
                )
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "Enabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.audio.game_audio.enabled = true",
                )
                .unwrap(),
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_name: "Enabled".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

#[cfg(target_os = "linux")]
pub fn microphone_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Headset microphone".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: [
            HigherOrderChoiceOption {
                name: "Disabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.audio.microphone.enabled = false",
                )
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "Enabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.audio.microphone.enabled = true",
                )
                .unwrap(),
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_name: "Enabled".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn game_audio_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Headset speaker".into(),
        strings: [(
            "notice".into(),
            "You can change the default audio device from the system taskbar tray (bottom right)"
                .into(),
        )]
        .into_iter()
        .collect(),
        flags: HashSet::new(),
        options: vec![
            HigherOrderChoiceOption {
                name: "Disabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.audio.game_audio.enabled = false",
                )
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "System Default".to_owned(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    r"session_settings.audio.game_audio.enabled = true
                    session_settings.audio.game_audio.content.device.set = false",
                )
                .unwrap(),
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_name: "System Default".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn microphone_schema() -> PresetSchemaNode {
    let mut microhone_options = vec![HigherOrderChoiceOption {
        name: "Disabled".to_owned(),
        modifiers: alvr_packets::parse_path_value_pairs(
            "session_settings.audio.microphone.enabled = false",
        )
        .unwrap(),
        content: None,
    }];

    if cfg!(windows) {
        for (key, display_name) in [
            ("Automatic", "Automatic"),
            ("VAC", "Virtual Audio Cable"),
            ("VBCable", "VB Cable"),
            ("VoiceMeeter", "VoiceMeeter"),
            ("VoiceMeeterAux", "VoiceMeeter Aux"),
            ("VoiceMeeterVaio3", "VoiceMeeter VAIO3"),
        ] {
            microhone_options.push(HigherOrderChoiceOption {
                name: display_name.into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r#"session_settings.audio.microphone.enabled = true
                    session_settings.audio.microphone.content.devices.variant = "{key}""#,
                ))
                .unwrap(),
                content: None,
            })
        }
    }

    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Headset microphone".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: microhone_options.into_iter().collect(),
        default_option_name: "Disabled".into(),
        gui: ChoiceControlType::Dropdown,
    })
}

pub fn hand_tracking_interaction_schema() -> PresetSchemaNode {
    const HELP: &str = r"Disabled: hands cannot emulate buttons. Useful for using Joy-Cons or other non-native controllers.
SteamVR Input 2.0: create separate SteamVR devices for hand tracking.
ALVR bindings: use ALVR hand tracking button bindings. Check the wiki for help.
";

    const PREFIX: &str = "session_settings.headset.controllers.content";

    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Hand tracking interaction".into(),
        strings: [("help".into(), HELP.into())].into_iter().collect(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            HigherOrderChoiceOption {
                name: "Disabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r"session_settings.headset.controllers.enabled = false
                    {PREFIX}.hand_skeleton.content.steamvr_input_2_0 = false
                    {PREFIX}.hand_tracking_interaction.enabled = false"
                ))
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "SteamVR Input 2.0".into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r"session_settings.headset.controllers.enabled = true
                    {PREFIX}.hand_skeleton.enabled = true
                    {PREFIX}.hand_skeleton.content.steamvr_input_2_0 = true
                    {PREFIX}.hand_tracking_interaction.enabled = false"
                ))
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "ALVR bindings".into(),
                modifiers: alvr_packets::parse_path_value_pairs(&format!(
                    r"session_settings.headset.controllers.enabled = true
                    {PREFIX}.hand_skeleton.content.steamvr_input_2_0 = false
                    {PREFIX}.hand_tracking_interaction.enabled = true"
                ))
                .unwrap(),
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_name: "SteamVR Input 2.0".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}

pub fn eye_face_tracking_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "Eye and face tracking".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: [
            HigherOrderChoiceOption {
                name: "Disabled".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    "session_settings.headset.face_tracking.enabled = false"
                )
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "VRChat Eye OSC".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    r#"session_settings.headset.face_tracking.enabled = true
                    session_settings.headset.face_tracking.content.sink.variant = "VrchatEyeOsc""#
                )
                .unwrap(),
                content: None,
            },
            HigherOrderChoiceOption {
                name: "VRCFaceTracking".into(),
                modifiers: alvr_packets::parse_path_value_pairs(
                    r#"session_settings.headset.face_tracking.enabled = true
                    session_settings.headset.face_tracking.content.sink.variant = "VrcFaceTracking""#
                )
                .unwrap(),
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_name: "Disabled".into(),
        gui: ChoiceControlType::ButtonGroup,
    })
}
