use super::schema::{
    HigherOrderChoiceOption, HigherOrderChoiceSchema, PresetModifier, PresetSchemaNode,
};
use crate::dashboard::components::presets::schema::PresetModifierOperation;
use settings_schema::ChoiceControlType;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

fn string_modifier(target_path: String, value: &str) -> PresetModifier {
    PresetModifier {
        target_path,
        operation: PresetModifierOperation::Assign(serde_json::Value::String(value.into())),
    }
}
fn uint_modifier(target_path: String, value: &str) -> PresetModifier {
    PresetModifier {
        target_path,
        operation: PresetModifierOperation::Assign(serde_json::Value::Number(
            serde_json::Number::from_str(value).unwrap(),
        )),
    }
}
fn bool_modifier(target_path: String, value: bool) -> PresetModifier {
    PresetModifier {
        target_path,
        operation: PresetModifierOperation::Assign(serde_json::Value::Bool(value)),
    }
}

pub fn resolution_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "resolution".into(),
        strings: HashMap::new(),
        flags: ["steamvr-restart".into()].into_iter().collect(),
        options: [
            ("Very Low", "1536"),
            ("Low", "1856"),
            ("Medium", "2144"),
            ("High", "2592"),
            ("Ultra", "2816"),
            ("Extreme", "3040"),
        ]
        .into_iter()
        .map(|(key, value)| HigherOrderChoiceOption {
            display_name: key.into(),
            modifiers: [
                string_modifier(
                    "session_settings.video.transcoding_resolution.variant".into(),
                    "Absolute",
                ),
                uint_modifier(
                    "session_settings.video.transcoding_resolution.Absolute.width".into(),
                    value,
                ),
                bool_modifier(
                    "session_settings.video.transcoding_resolution.Absolute.height.set".into(),
                    false,
                ),
                string_modifier(
                    "session_settings.video.emulated_headset_resolution.variant".into(),
                    "Absolute",
                ),
                uint_modifier(
                    "session_settings.video.emulated_headset_resolution.Absolute.width".into(),
                    value,
                ),
                bool_modifier(
                    "session_settings.video.emulated_headset_resolution.Absolute.height.set".into(),
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

pub fn game_audio_schema(devices: Vec<String>) -> PresetSchemaNode {
    let mut game_audio_options = vec![
        HigherOrderChoiceOption {
            display_name: "Disabled".into(),
            modifiers: vec![bool_modifier(
                "session_settings.audio.game_audio.enabled".into(),
                false,
            )],
            content: None,
        },
        HigherOrderChoiceOption {
            display_name: "System Default".to_owned(),
            modifiers: vec![
                bool_modifier("session_settings.audio.game_audio.enabled".into(), true),
                bool_modifier(
                    "session_settings.audio.game_audio.content.device.set".into(),
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
                bool_modifier("session_settings.audio.game_audio.enabled".into(), true),
                bool_modifier(
                    "session_settings.audio.game_audio.content.device.set".into(),
                    true,
                ),
                string_modifier(
                    "session_settings.audio.game_audio.content.device.content.variant".into(),
                    "NameSubstring",
                ),
                string_modifier(
                    "session_settings.audio.game_audio.content.device.content.NameSubstring".into(),
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
            "session_settings.audio.microphone.enabled".into(),
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
                    bool_modifier("session_settings.audio.microphone.enabled".into(), true),
                    string_modifier(
                        "session_settings.audio.microphone.content.devices.variant".into(),
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
                    bool_modifier("session_settings.audio.microphone.enabled".into(), true),
                    string_modifier(format!("{PREFIX}.variant"), "Custom"),
                    string_modifier(format!("{PREFIX}.Custom.sink.variant"), "NameSubstring"),
                    string_modifier(format!("{PREFIX}.Custom.sink.NameSubstring"), &name),
                    // string_modifier(format!("{PREFIX}.Custom.source.variant"), "NameSubstring"),
                    // string_modifier(format!("{PREFIX}.Custom.source.NameSubstring"), ""),
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

pub fn null_preset_schema() -> PresetSchemaNode {
    PresetSchemaNode::HigherOrderChoice(HigherOrderChoiceSchema {
        name: "null".into(),
        strings: HashMap::new(),
        flags: HashSet::new(),
        options: vec![HigherOrderChoiceOption {
            display_name: "null".into(),
            modifiers: vec![],
            content: None,
        }],
        default_option_index: 0,
        gui: ChoiceControlType::Dropdown,
    })
}
