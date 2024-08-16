use alvr_common::*;
use alvr_packets::{ButtonEntry, ButtonValue};
use alvr_session::{
    AutomaticButtonMappingConfig, BinaryToScalarStates, ButtonBindingTarget, ButtonMappingType,
    ControllersEmulationMode, HysteresisThreshold, Range,
};
use std::collections::{HashMap, HashSet};

pub fn registered_button_set(
    controllers_emulation_mode: &ControllersEmulationMode,
) -> HashSet<u64> {
    match &controllers_emulation_mode {
        ControllersEmulationMode::RiftSTouch
        | ControllersEmulationMode::Quest2Touch
        | ControllersEmulationMode::Quest3Plus => CONTROLLER_PROFILE_INFO
            .get(&QUEST_CONTROLLER_PROFILE_ID)
            .unwrap()
            .button_set
            .clone(),
        ControllersEmulationMode::ValveIndex => CONTROLLER_PROFILE_INFO
            .get(&INDEX_CONTROLLER_PROFILE_ID)
            .unwrap()
            .button_set
            .clone(),
        ControllersEmulationMode::ViveWand => CONTROLLER_PROFILE_INFO
            .get(&VIVE_CONTROLLER_PROFILE_ID)
            .unwrap()
            .button_set
            .clone(),
        ControllersEmulationMode::ViveTracker => HashSet::new(),
        ControllersEmulationMode::Custom { button_set, .. } => button_set
            .iter()
            .map(|b| alvr_common::hash_string(b))
            .collect(),
    }
}

pub struct BindingTarget {
    destination: u64,
    mapping_type: ButtonMappingType,
    binary_conditions: Vec<u64>,
}

// Inputs relative to the same physical button
#[derive(Clone, Copy)]
pub struct ButtonInputs {
    click: Option<u64>,
    touch: Option<u64>,
    value: Option<u64>,
    force: Option<u64>,
}

fn click(click: u64) -> ButtonInputs {
    ButtonInputs {
        click: Some(click),
        touch: None,
        value: None,
        force: None,
    }
}

fn ct(set: &HashSet<u64>, click: u64, touch: u64) -> ButtonInputs {
    ButtonInputs {
        click: Some(click),
        touch: set.contains(&touch).then_some(touch),
        value: None,
        force: None,
    }
}

fn value(value: u64) -> ButtonInputs {
    ButtonInputs {
        click: None,
        touch: None,
        value: Some(value),
        force: None,
    }
}

fn ctv(set: &HashSet<u64>, click: u64, touch: u64, value: u64) -> ButtonInputs {
    ButtonInputs {
        click: set.contains(&click).then_some(click),
        touch: set.contains(&touch).then_some(touch),
        value: set.contains(&value).then_some(value),
        force: None,
    }
}

fn ctvf(set: &HashSet<u64>, click: u64, touch: u64, value: u64, force: u64) -> ButtonInputs {
    ButtonInputs {
        click: set.contains(&click).then_some(click),
        touch: set.contains(&touch).then_some(touch),
        value: set.contains(&value).then_some(value),
        force: set.contains(&force).then_some(force),
    }
}

fn passthrough(target: u64) -> BindingTarget {
    BindingTarget {
        destination: target,
        mapping_type: ButtonMappingType::Passthrough,
        binary_conditions: vec![],
    }
}

fn binary_to_scalar(target: u64, map: BinaryToScalarStates) -> BindingTarget {
    BindingTarget {
        destination: target,
        mapping_type: ButtonMappingType::BinaryToScalar(map),
        binary_conditions: vec![],
    }
}

fn hysteresis_threshold(target: u64, map: HysteresisThreshold) -> BindingTarget {
    BindingTarget {
        destination: target,
        mapping_type: ButtonMappingType::HysteresisThreshold(map),
        binary_conditions: vec![],
    }
}

fn remap(target: u64, map: Range) -> BindingTarget {
    BindingTarget {
        destination: target,
        mapping_type: ButtonMappingType::Remap(map),
        binary_conditions: vec![],
    }
}

// Map two buttons with eterogeneous inputs
fn map_button_pair_automatic(
    source: ButtonInputs,
    destination: ButtonInputs,
    config: &AutomaticButtonMappingConfig,
) -> impl Iterator<Item = (u64, Vec<BindingTarget>)> {
    let click_to_value = BinaryToScalarStates { off: 0.0, on: 1.0 };

    let mut entries = vec![];
    if let Some(source_click) = source.click {
        let mut targets = vec![];

        if let Some(destination_click) = destination.click {
            targets.push(passthrough(destination_click));
        }
        if source.touch.is_none() {
            if let Some(destination_touch) = destination.touch {
                targets.push(passthrough(destination_touch));
            }
        }
        if source.value.is_none() {
            if let Some(destination_value) = destination.value {
                targets.push(binary_to_scalar(destination_value, click_to_value));
            }
        }

        entries.push((source_click, targets));
    }
    if let Some(source_touch) = source.touch {
        let mut targets = vec![];
        if let Some(destination_touch) = destination.touch {
            targets.push(passthrough(destination_touch));
        }
        entries.push((source_touch, targets));
    }
    if let Some(source_value) = source.value {
        let mut targets = vec![];
        let mut remap_for_touch = false;
        let mut remap_for_force = false;

        if source.click.is_none() {
            if let Some(destination_click) = destination.click {
                targets.push(hysteresis_threshold(
                    destination_click,
                    config.click_threshold,
                ));
            }
        }
        if source.touch.is_none() {
            if let Some(destination_touch) = destination.touch {
                targets.push(hysteresis_threshold(
                    destination_touch,
                    config.touch_threshold,
                ));
                remap_for_touch = true;
            }
        }
        if source.force.is_none() {
            if let Some(destination_force) = destination.force {
                targets.push(remap(
                    destination_force,
                    Range {
                        min: config.force_threshold,
                        max: 1.0,
                    },
                ));
                remap_for_force = true;
            }
        }
        if let Some(destination_value) = destination.value {
            if !remap_for_touch && !remap_for_force {
                targets.push(passthrough(destination_value));
            } else {
                let low = if remap_for_touch {
                    config.touch_threshold.value
                } else {
                    0.0
                };
                let high = if remap_for_force {
                    config.force_threshold
                } else {
                    1.0
                };
                targets.push(remap(
                    destination_value,
                    Range {
                        min: low,
                        max: high,
                    },
                ));
            }
        }

        entries.push((source_value, targets));
    }

    entries.into_iter()
}

pub fn automatic_bindings(
    source_set: &HashSet<u64>,
    destination_set: &HashSet<u64>,
    config: &AutomaticButtonMappingConfig,
) -> HashMap<u64, Vec<BindingTarget>> {
    let s_set = source_set;
    let d_set = destination_set;

    let mut bindings = HashMap::new();

    // Menu buttons
    if s_set.contains(&*LEFT_MENU_CLICK_ID) {
        let click = click(*LEFT_MENU_CLICK_ID);
        if d_set.contains(&*LEFT_MENU_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(click, click, config));
        } else if d_set.contains(&*LEFT_SYSTEM_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                click,
                ct(s_set, *LEFT_SYSTEM_CLICK_ID, *LEFT_SYSTEM_TOUCH_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_MENU_CLICK_ID) {
        let click = click(*RIGHT_MENU_CLICK_ID);
        if d_set.contains(&*RIGHT_MENU_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(click, click, config));
        } else if d_set.contains(&*RIGHT_SYSTEM_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                click,
                ct(s_set, *RIGHT_SYSTEM_CLICK_ID, *RIGHT_SYSTEM_TOUCH_ID),
                config,
            ));
        }
    }

    // A/X buttons
    if s_set.contains(&*LEFT_X_CLICK_ID) {
        let source = ct(s_set, *LEFT_X_CLICK_ID, *LEFT_X_TOUCH_ID);
        if d_set.contains(&*LEFT_X_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_X_CLICK_ID, *LEFT_X_TOUCH_ID),
                config,
            ));
        } else if d_set.contains(&*LEFT_A_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_A_CLICK_ID, *LEFT_A_TOUCH_ID),
                config,
            ));
        } else if d_set.contains(&*LEFT_TRACKPAD_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_TRACKPAD_CLICK_ID, *LEFT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_A_CLICK_ID) {
        let source = ct(s_set, *RIGHT_A_CLICK_ID, *RIGHT_A_TOUCH_ID);
        if d_set.contains(&*RIGHT_A_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *RIGHT_A_CLICK_ID, *RIGHT_A_TOUCH_ID),
                config,
            ));
        } else if d_set.contains(&*RIGHT_TRACKPAD_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *RIGHT_TRACKPAD_CLICK_ID, *RIGHT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }

    // B/Y buttons
    if s_set.contains(&*LEFT_Y_CLICK_ID) {
        let source = ct(s_set, *LEFT_Y_CLICK_ID, *LEFT_Y_TOUCH_ID);
        if d_set.contains(&*LEFT_Y_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_Y_CLICK_ID, *LEFT_Y_TOUCH_ID),
                config,
            ));
        } else if d_set.contains(&*LEFT_B_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_B_CLICK_ID, *LEFT_B_TOUCH_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_B_CLICK_ID) && d_set.contains(&*RIGHT_B_CLICK_ID) {
        bindings.extend(map_button_pair_automatic(
            ct(s_set, *RIGHT_B_CLICK_ID, *RIGHT_B_TOUCH_ID),
            ct(d_set, *RIGHT_B_CLICK_ID, *RIGHT_B_TOUCH_ID),
            config,
        ));
    }

    // Squeeze buttons
    if (s_set.contains(&*LEFT_SQUEEZE_CLICK_ID) || s_set.contains(&*LEFT_SQUEEZE_VALUE_ID))
        && (d_set.contains(&*LEFT_SQUEEZE_CLICK_ID) || d_set.contains(&*LEFT_SQUEEZE_VALUE_ID))
    {
        bindings.extend(map_button_pair_automatic(
            ctvf(
                s_set,
                *LEFT_SQUEEZE_CLICK_ID,
                *LEFT_SQUEEZE_TOUCH_ID,
                *LEFT_SQUEEZE_VALUE_ID,
                *LEFT_SQUEEZE_FORCE_ID,
            ),
            ctvf(
                d_set,
                *LEFT_SQUEEZE_CLICK_ID,
                *LEFT_SQUEEZE_TOUCH_ID,
                *LEFT_SQUEEZE_VALUE_ID,
                *LEFT_SQUEEZE_FORCE_ID,
            ),
            config,
        ));
    }
    if (s_set.contains(&*RIGHT_SQUEEZE_CLICK_ID) || s_set.contains(&*RIGHT_SQUEEZE_VALUE_ID))
        && (d_set.contains(&*RIGHT_SQUEEZE_CLICK_ID) || d_set.contains(&*RIGHT_SQUEEZE_VALUE_ID))
    {
        bindings.extend(map_button_pair_automatic(
            ctvf(
                s_set,
                *RIGHT_SQUEEZE_CLICK_ID,
                *RIGHT_SQUEEZE_TOUCH_ID,
                *RIGHT_SQUEEZE_VALUE_ID,
                *RIGHT_SQUEEZE_FORCE_ID,
            ),
            ctvf(
                d_set,
                *RIGHT_SQUEEZE_CLICK_ID,
                *RIGHT_SQUEEZE_TOUCH_ID,
                *RIGHT_SQUEEZE_VALUE_ID,
                *RIGHT_SQUEEZE_FORCE_ID,
            ),
            config,
        ));
    }

    // Trigger buttons
    if (s_set.contains(&*LEFT_TRIGGER_CLICK_ID) || s_set.contains(&*LEFT_TRIGGER_VALUE_ID))
        && (d_set.contains(&*LEFT_TRIGGER_CLICK_ID) || d_set.contains(&*LEFT_TRIGGER_VALUE_ID))
    {
        bindings.extend(map_button_pair_automatic(
            ctv(
                s_set,
                *LEFT_TRIGGER_CLICK_ID,
                *LEFT_TRIGGER_TOUCH_ID,
                *LEFT_TRIGGER_VALUE_ID,
            ),
            ctv(
                d_set,
                *LEFT_TRIGGER_CLICK_ID,
                *LEFT_TRIGGER_TOUCH_ID,
                *LEFT_TRIGGER_VALUE_ID,
            ),
            config,
        ));
    }
    if (s_set.contains(&*RIGHT_TRIGGER_CLICK_ID) || s_set.contains(&*RIGHT_TRIGGER_VALUE_ID))
        && (d_set.contains(&*RIGHT_TRIGGER_CLICK_ID) || d_set.contains(&*RIGHT_TRIGGER_VALUE_ID))
    {
        bindings.extend(map_button_pair_automatic(
            ctv(
                s_set,
                *RIGHT_TRIGGER_CLICK_ID,
                *RIGHT_TRIGGER_TOUCH_ID,
                *RIGHT_TRIGGER_VALUE_ID,
            ),
            ctv(
                d_set,
                *RIGHT_TRIGGER_CLICK_ID,
                *RIGHT_TRIGGER_TOUCH_ID,
                *RIGHT_TRIGGER_VALUE_ID,
            ),
            config,
        ));
    }

    // Thumbsticks
    if s_set.contains(&*LEFT_THUMBSTICK_X_ID) {
        let x = value(*LEFT_THUMBSTICK_X_ID);
        let y = value(*LEFT_THUMBSTICK_Y_ID);
        if d_set.contains(&*LEFT_THUMBSTICK_X_ID) {
            bindings.extend(map_button_pair_automatic(x, x, config));
            bindings.extend(map_button_pair_automatic(y, y, config));
        } else if d_set.contains(&*LEFT_TRACKPAD_X_ID) {
            bindings.extend(map_button_pair_automatic(
                x,
                value(*LEFT_TRACKPAD_X_ID),
                config,
            ));
            bindings.extend(map_button_pair_automatic(
                y,
                value(*LEFT_TRACKPAD_Y_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*LEFT_THUMBSTICK_CLICK_ID) {
        let source = ct(s_set, *LEFT_THUMBSTICK_CLICK_ID, *LEFT_THUMBSTICK_TOUCH_ID);
        if d_set.contains(&*LEFT_THUMBSTICK_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_THUMBSTICK_CLICK_ID, *LEFT_THUMBSTICK_TOUCH_ID),
                config,
            ));
        } else if d_set.contains(&*LEFT_TRACKPAD_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *LEFT_TRACKPAD_CLICK_ID, *LEFT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_THUMBSTICK_X_ID) {
        let x = value(*RIGHT_THUMBSTICK_X_ID);
        let y = value(*RIGHT_THUMBSTICK_Y_ID);
        if d_set.contains(&*RIGHT_THUMBSTICK_X_ID) {
            bindings.extend(map_button_pair_automatic(x, x, config));
            bindings.extend(map_button_pair_automatic(y, y, config));
        } else if d_set.contains(&*RIGHT_TRACKPAD_X_ID) {
            bindings.extend(map_button_pair_automatic(
                x,
                value(*RIGHT_TRACKPAD_X_ID),
                config,
            ));
            bindings.extend(map_button_pair_automatic(
                y,
                value(*RIGHT_TRACKPAD_Y_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_THUMBSTICK_CLICK_ID) {
        let source = ct(
            s_set,
            *RIGHT_THUMBSTICK_CLICK_ID,
            *RIGHT_THUMBSTICK_TOUCH_ID,
        );
        if d_set.contains(&*RIGHT_THUMBSTICK_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(
                    d_set,
                    *RIGHT_THUMBSTICK_CLICK_ID,
                    *RIGHT_THUMBSTICK_TOUCH_ID,
                ),
                config,
            ));
        } else if d_set.contains(&*RIGHT_TRACKPAD_CLICK_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                ct(d_set, *RIGHT_TRACKPAD_CLICK_ID, *RIGHT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }

    // Thumbrests
    if s_set.contains(&*LEFT_THUMBREST_TOUCH_ID) {
        let source = value(*LEFT_THUMBREST_TOUCH_ID);
        if d_set.contains(&*LEFT_THUMBREST_TOUCH_ID) {
            bindings.extend(map_button_pair_automatic(source, source, config));
        } else if d_set.contains(&*LEFT_TRACKPAD_TOUCH_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                value(*LEFT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }
    if s_set.contains(&*RIGHT_THUMBREST_TOUCH_ID) {
        let source = value(*RIGHT_THUMBREST_TOUCH_ID);
        if d_set.contains(&*RIGHT_THUMBREST_TOUCH_ID) {
            bindings.extend(map_button_pair_automatic(source, source, config));
        } else if d_set.contains(&*RIGHT_TRACKPAD_TOUCH_ID) {
            bindings.extend(map_button_pair_automatic(
                source,
                value(*RIGHT_TRACKPAD_TOUCH_ID),
                config,
            ));
        }
    }

    bindings
}

pub struct ButtonMappingManager {
    mappings: HashMap<u64, Vec<BindingTarget>>,
    binary_source_states: HashMap<u64, bool>,
    hysteresis_states: HashMap<u64, HashMap<u64, bool>>,
}

impl ButtonMappingManager {
    pub fn new_automatic(
        source: &HashSet<u64>,
        controllers_emulation_mode: &ControllersEmulationMode,
        button_mapping_config: &AutomaticButtonMappingConfig,
    ) -> Self {
        let button_set = registered_button_set(controllers_emulation_mode);
        Self {
            mappings: automatic_bindings(source, &button_set, button_mapping_config),
            binary_source_states: HashMap::new(),
            hysteresis_states: HashMap::new(),
        }
    }

    pub fn new_manual(mappings: &[(String, Vec<ButtonBindingTarget>)]) -> Self {
        let mappings = mappings
            .iter()
            .map(|(key, value)| {
                (
                    alvr_common::hash_string(key),
                    value
                        .iter()
                        .map(|b| BindingTarget {
                            destination: alvr_common::hash_string(&b.destination),
                            mapping_type: b.mapping_type.clone(),
                            binary_conditions: b
                                .binary_conditions
                                .iter()
                                .map(|c| alvr_common::hash_string(c))
                                .collect(),
                        })
                        .collect(),
                )
            })
            .collect();

        Self {
            mappings,
            binary_source_states: HashMap::new(),
            hysteresis_states: HashMap::new(),
        }
    }

    // Apply any button changes that are mapped to this specific button
    pub fn map_button(&mut self, source_button: &ButtonEntry) -> Vec<ButtonEntry> {
        if let ButtonValue::Binary(value) = source_button.value {
            let val_ref = self
                .binary_source_states
                .entry(source_button.path_id)
                .or_default();

            if value == *val_ref {
                return vec![];
            }

            // NB: Update value
            *val_ref = value;
        }

        let mut destination_buttons = vec![];

        if let Some(mappings) = self.mappings.get(&source_button.path_id) {
            'mapping: for mapping in mappings {
                let destination_value = match (&mapping.mapping_type, source_button.value) {
                    (ButtonMappingType::Passthrough, value) => value,
                    (
                        ButtonMappingType::HysteresisThreshold(threshold),
                        ButtonValue::Scalar(value),
                    ) => {
                        let state = self
                            .hysteresis_states
                            .entry(source_button.path_id)
                            .or_default()
                            .entry(mapping.destination)
                            .or_default();

                        if *state && value < threshold.value - threshold.deviation {
                            *state = false;
                        } else if !*state && value > threshold.value + threshold.deviation {
                            *state = true;
                        } else {
                            // No change needed
                            continue;
                        }

                        ButtonValue::Binary(*state)
                    }
                    (ButtonMappingType::BinaryToScalar(levels), ButtonValue::Binary(value)) => {
                        if value {
                            ButtonValue::Scalar(levels.on)
                        } else {
                            ButtonValue::Scalar(levels.off)
                        }
                    }
                    (ButtonMappingType::Remap(range), ButtonValue::Scalar(value)) => {
                        let value = (value - range.min) / (range.max - range.min);
                        ButtonValue::Scalar(value.clamp(0.0, 1.0))
                    }
                    _ => {
                        error!("Failed to map button!");
                        continue;
                    }
                };

                for source_id in &mapping.binary_conditions {
                    if !self
                        .binary_source_states
                        .get(source_id)
                        .copied()
                        .unwrap_or(false)
                    {
                        continue 'mapping;
                    }
                }

                destination_buttons.push(ButtonEntry {
                    path_id: mapping.destination,
                    value: destination_value,
                });
            }
        } else {
            let button_name = BUTTON_INFO
                .get(&source_button.path_id)
                .map(|info| info.path)
                .unwrap_or("Unknown");
            info!("Received button not mapped: {button_name}");
        }

        destination_buttons
    }
}
