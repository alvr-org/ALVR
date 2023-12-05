use super::schema::{
    HigherOrderChoiceOption, HigherOrderChoiceSchema, PresetModifier, PresetSchemaNode,
};
use crate::dashboard::components::presets::schema::PresetModifierOperation;
use settings_schema::ChoiceControlType;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

fn string_modifier(target_path: &str, value: &str) -> PresetModifier {
    PresetModifier {
        target_path: target_path.into(),
        operation: PresetModifierOperation::Assign(serde_json::Value::String(value.into())),
    }
}
fn num_modifier(target_path: &str, value: &str) -> PresetModifier {
    PresetModifier {
        target_path: target_path.into(),
        operation: PresetModifierOperation::Assign(serde_json::Value::Number(
            serde_json::Number::from_str(value).unwrap(),
        )),
    }
}
fn bool_modifier(target_path: &str, value: bool) -> PresetModifier {
    PresetModifier {
        target_path: target_path.into(),
        operation: PresetModifierOperation::Assign(serde_json::Value::Bool(value)),
    }
}

pub fn resolution_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "resolution".into(),
        strings: HashMap::new(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            ("Very Low (width: 3072)", "1536"),
            ("Low (width: 3712)", "1856"),
            ("Medium (width: 4288)", "2144"),
            ("High (width: 5184)", "2592"),
            ("Ultra (width: 5632)", "2816"),
            ("Extreme (width: 6080)", "3040"),
        ]
        .into_iter()
        .map(|(key, value)| HigherOrderChoiceOption {
            display_name: key.into(),
            modifiers: [
                string_modifier(
                    "session_settings.video.transcoding_view_resolution.variant",
                    "Absolute",
                ),
                num_modifier(
                    "session_settings.video.transcoding_view_resolution.Absolute.width",
                    value,
                ),
                bool_modifier(
                    "session_settings.video.transcoding_view_resolution.Absolute.height.set",
                    false,
                ),
                string_modifier(
                    "session_settings.video.emulated_headset_view_resolution.variant",
                    "Absolute",
                ),
                num_modifier(
                    "session_settings.video.emulated_headset_view_resolution.Absolute.width",
                    value,
                ),
                bool_modifier(
                    "session_settings.video.emulated_headset_view_resolution.Absolute.height.set",
                    false,
                ),
            ]
            .into_iter()
            .collect(),
            content: None,
        })
        .collect(),
        default_option_index: 2,
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
                display_name: format!("{framerate}Hz"),
                modifiers: [num_modifier(
                    "session_settings.video.preferred_fps",
                    &format!("{:?}", framerate as f32),
                )]
                .into_iter()
                .collect(),
                content: None,
            })
            .collect(),
        default_option_index: 1,
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
            display_name: key.into(),
            modifiers: [
                string_modifier(
                    "session_settings.video.encoder_config.nvenc.quality_preset.variant",
                    val_nv,
                ),
                string_modifier(
                    "session_settings.video.encoder_config.amf.quality_preset.variant",
                    val_amd,
                ),
            ]
            .into_iter()
            .collect(),
            content: None,
        })
        .collect(),
        default_option_index: 0,
        gui: ChoiceControlType::ButtonGroup,
    })
}

pub fn game_audio_schema(devices: Vec<String>) -> PresetSchemaNode {
    let mut game_audio_options = vec![
        HigherOrderChoiceOption {
            display_name: "Disabled".into(),
            modifiers: vec![bool_modifier(
                "session_settings.audio.game_audio.enabled",
                false,
            )],
            content: None,
        },
        HigherOrderChoiceOption {
            display_name: "System Default".to_owned(),
            modifiers: vec![
                bool_modifier("session_settings.audio.game_audio.enabled", true),
                bool_modifier(
                    "session_settings.audio.game_audio.content.device.set",
                    false,
                ),
            ],
            content: None,
        },
    ];

    for name in devices {
        game_audio_options.push(HigherOrderChoiceOption {
            display_name: name.clone(),
            modifiers: vec![
                bool_modifier("session_settings.audio.game_audio.enabled", true),
                bool_modifier("session_settings.audio.game_audio.content.device.set", true),
                string_modifier(
                    "session_settings.audio.game_audio.content.device.content.variant",
                    "NameSubstring",
                ),
                string_modifier(
                    "session_settings.audio.game_audio.content.device.content.NameSubstring",
                    &name,
                ),
            ],
            content: None,
        })
    }

    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "game_audio".into(),
        strings: [(
            "help".into(),
            "You should keep this as default. Change the default audio device from the global OS settings".into(),
        )]
        .into_iter()
        .collect(),
        flags: HashSet::new(),
        options: game_audio_options.into_iter().collect(),
        default_option_index: 1,
        gui: ChoiceControlType::Dropdown,
    })
}

pub fn microphone_schema(devices: Vec<String>) -> PresetSchemaNode {
    let mut microhone_options = vec![HigherOrderChoiceOption {
        display_name: "Disabled".to_owned(),
        modifiers: vec![bool_modifier(
            "session_settings.audio.microphone.enabled",
            false,
        )],
        content: None,
    }];

    if cfg!(windows) {
        for (key, display_name) in [
            ("Automatic", "Automatic"),
            ("VBCable", "VB Cable"),
            ("VoiceMeeter", "VoiceMeeter"),
            ("VoiceMeeterAux", "VoiceMeeter Aux"),
            ("VoiceMeeterVaio3", "VoiceMeeter VAIO3"),
        ] {
            microhone_options.push(HigherOrderChoiceOption {
                display_name: display_name.into(),
                modifiers: vec![
                    bool_modifier("session_settings.audio.microphone.enabled", true),
                    string_modifier(
                        "session_settings.audio.microphone.content.devices.variant",
                        key,
                    ),
                ],
                content: None,
            })
        }
    } else {
        const PREFIX: &str = "session_settings.audio.microphone.content.devices";
        for name in devices {
            microhone_options.push(HigherOrderChoiceOption {
                display_name: name.clone(),
                modifiers: vec![
                    bool_modifier("session_settings.audio.microphone.enabled", true),
                    string_modifier(&format!("{PREFIX}.variant"), "Custom"),
                    string_modifier(&format!("{PREFIX}.Custom.sink.variant"), "NameSubstring"),
                    string_modifier(&format!("{PREFIX}.Custom.sink.NameSubstring"), &name),
                ],
                content: None,
            })
        }
    };

    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "microphone".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: microhone_options.into_iter().collect(),
        default_option_index: 0,
        gui: ChoiceControlType::Dropdown,
    })
}

pub fn eye_face_tracking_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "eye_face_tracking".into(),
        strings: [("display_name".into(), "Eye and face tracking".into())]
            .into_iter()
            .collect(),
        flags: HashSet::new(),
        options: [
            HigherOrderChoiceOption {
                display_name: "VRChat Eye OSC".into(),
                modifiers: vec![
                    bool_modifier("session_settings.headset.face_tracking.enabled", true),
                    string_modifier(
                        "session_settings.headset.face_tracking.content.sink.variant",
                        "VrchatEyeOsc",
                    ),
                ],
                content: None,
            },
            HigherOrderChoiceOption {
                display_name: "VRCFaceTracking".into(),
                modifiers: vec![
                    bool_modifier("session_settings.headset.face_tracking.enabled", true),
                    string_modifier(
                        "session_settings.headset.face_tracking.content.sink.variant",
                        "VrcFaceTracking",
                    ),
                ],
                content: None,
            },
            HigherOrderChoiceOption {
                display_name: "Disable".into(),
                modifiers: vec![bool_modifier(
                    "session_settings.headset.face_tracking.enabled",
                    false,
                )],
                content: None,
            },
        ]
        .into_iter()
        .collect(),
        default_option_index: 2,
        gui: ChoiceControlType::ButtonGroup,
    })
}
