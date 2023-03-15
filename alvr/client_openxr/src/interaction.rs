use crate::{to_pose, to_quat, to_vec3, Platform};
use alvr_common::{glam::Vec3, *};
use alvr_events::ButtonValue;
use alvr_sockets::{DeviceMotion, Pose};
use openxr as xr;
use std::collections::HashMap;

enum BindingType {
    Binary,
    Scalar,
}

pub enum ButtonAction {
    Binary(xr::Action<bool>),
    Scalar(xr::Action<f32>),
}

pub struct HandSource {
    pub grip_action: xr::Action<xr::Posef>,
    pub grip_space: xr::Space,
    pub skeleton_tracker: Option<xr::HandTracker>,
    pub vibration_action: xr::Action<xr::Haptic>,
}

pub struct ButtonBindingInfo {
    name: String,
    //note: this might be different than the path that generated the id
    binding_path: String,
    binding_type: BindingType,
}

const QUEST_CONTROLLER_PROFILE: &str = "/interaction_profiles/oculus/touch_controller";
const PICO_CONTROLLER_PROFILE: &str = "/interaction_profiles/pico/neo3_controller";
const FOCUS3_CONTROLLER_PROFILE: &str = "/interaction_profiles/htc/vive_focus3_controller";
const YVR_CONTROLLER_PROFILE: &str = "/interaction_profiles/yvr/touch_controller";

fn get_button_bindings(platform: Platform) -> HashMap<u64, ButtonBindingInfo> {
    let mut map = HashMap::new();

    // Baseline bindings for the Touch controller
    map.insert(
        *MENU_CLICK_ID,
        ButtonBindingInfo {
            name: "menu_click".into(),
            binding_path: MENU_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *A_CLICK_ID,
        ButtonBindingInfo {
            name: "a_click".into(),
            binding_path: A_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *A_TOUCH_ID,
        ButtonBindingInfo {
            name: "a_touch".into(),
            binding_path: A_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *B_CLICK_ID,
        ButtonBindingInfo {
            name: "b_click".into(),
            binding_path: B_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *B_TOUCH_ID,
        ButtonBindingInfo {
            name: "b_touch".into(),
            binding_path: B_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *X_CLICK_ID,
        ButtonBindingInfo {
            name: "x_click".into(),
            binding_path: X_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *X_TOUCH_ID,
        ButtonBindingInfo {
            name: "x_touch".into(),
            binding_path: X_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *Y_CLICK_ID,
        ButtonBindingInfo {
            name: "y_click".into(),
            binding_path: Y_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *Y_TOUCH_ID,
        ButtonBindingInfo {
            name: "y_touch".into(),
            binding_path: Y_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_SQUEEZE_VALUE_ID,
        ButtonBindingInfo {
            name: "left_squeeze_value".into(),
            binding_path: LEFT_SQUEEZE_VALUE_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *LEFT_SQUEEZE_CLICK_ID,
        ButtonBindingInfo {
            name: "left_squeeze_click".into(),
            binding_path: "/user/hand/left/input/squeeze".into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_TRIGGER_VALUE_ID,
        ButtonBindingInfo {
            name: "left_trigger_value".into(),
            binding_path: LEFT_TRIGGER_VALUE_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *LEFT_TRIGGER_CLICK_ID,
        ButtonBindingInfo {
            name: "left_trigger_click".into(),
            binding_path: "/user/hand/left/input/trigger".into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_TRIGGER_TOUCH_ID,
        ButtonBindingInfo {
            name: "left_trigger_touch".into(),
            binding_path: LEFT_TRIGGER_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_THUMBSTICK_X_ID,
        ButtonBindingInfo {
            name: "left_thumbstick_x".into(),
            binding_path: LEFT_THUMBSTICK_X_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *LEFT_THUMBSTICK_Y_ID,
        ButtonBindingInfo {
            name: "left_thumbstick_y".into(),
            binding_path: LEFT_THUMBSTICK_Y_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *LEFT_THUMBSTICK_CLICK_ID,
        ButtonBindingInfo {
            name: "left_thumbstick_click".into(),
            binding_path: LEFT_THUMBSTICK_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_THUMBSTICK_TOUCH_ID,
        ButtonBindingInfo {
            name: "left_thumbstick_touch".into(),
            binding_path: LEFT_THUMBSTICK_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *LEFT_THUMBREST_TOUCH_ID,
        ButtonBindingInfo {
            name: "left_thumbrest_touch".into(),
            binding_path: LEFT_THUMBREST_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_SQUEEZE_VALUE_ID,
        ButtonBindingInfo {
            name: "right_squeeze_value".into(),
            binding_path: RIGHT_SQUEEZE_VALUE_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *RIGHT_SQUEEZE_CLICK_ID,
        ButtonBindingInfo {
            name: "right_squeeze_click".into(),
            binding_path: "/user/hand/right/input/squeeze".into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_TRIGGER_VALUE_ID,
        ButtonBindingInfo {
            name: "right_trigger_value".into(),
            binding_path: RIGHT_TRIGGER_VALUE_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *RIGHT_TRIGGER_CLICK_ID,
        ButtonBindingInfo {
            name: "right_trigger_click".into(),
            binding_path: "/user/hand/right/input/trigger".into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_TRIGGER_TOUCH_ID,
        ButtonBindingInfo {
            name: "right_trigger_touch".into(),
            binding_path: RIGHT_TRIGGER_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_THUMBSTICK_X_ID,
        ButtonBindingInfo {
            name: "right_thumbstick_x".into(),
            binding_path: RIGHT_THUMBSTICK_X_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *RIGHT_THUMBSTICK_Y_ID,
        ButtonBindingInfo {
            name: "right_thumbstick_y".into(),
            binding_path: RIGHT_THUMBSTICK_Y_PATH.into(),
            binding_type: BindingType::Scalar,
        },
    );
    map.insert(
        *RIGHT_THUMBSTICK_CLICK_ID,
        ButtonBindingInfo {
            name: "right_thumbstick_click".into(),
            binding_path: RIGHT_THUMBSTICK_CLICK_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_THUMBSTICK_TOUCH_ID,
        ButtonBindingInfo {
            name: "right_thumbstick_touch".into(),
            binding_path: RIGHT_THUMBSTICK_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );
    map.insert(
        *RIGHT_THUMBREST_TOUCH_ID,
        ButtonBindingInfo {
            name: "right_thumbrest_touch".into(),
            binding_path: RIGHT_THUMBREST_TOUCH_PATH.into(),
            binding_type: BindingType::Binary,
        },
    );

    // Tweak bindings if other platforms
    if platform == Platform::Pico {
        map.insert(
            *MENU_CLICK_ID, // faked as oculus menu button
            ButtonBindingInfo {
                name: "back_click".into(),
                binding_path: BACK_CLICK_PATH.into(),
                binding_type: BindingType::Binary,
            },
        );
        map.remove(&*LEFT_THUMBREST_TOUCH_ID);
        map.remove(&*RIGHT_THUMBREST_TOUCH_ID);
    } else if platform == Platform::Vive {
        map.remove(&*A_TOUCH_ID);
        map.remove(&*B_TOUCH_ID);
        map.remove(&*X_TOUCH_ID);
        map.remove(&*Y_TOUCH_ID);
        map.remove(&*LEFT_SQUEEZE_CLICK_ID);
        map.remove(&*LEFT_TRIGGER_CLICK_ID);
        map.remove(&*LEFT_THUMBREST_TOUCH_ID);
        map.remove(&*RIGHT_SQUEEZE_CLICK_ID);
        map.remove(&*RIGHT_TRIGGER_CLICK_ID);
        map.remove(&*RIGHT_THUMBREST_TOUCH_ID);
    } else if platform == Platform::Yvr {
        map.remove(&*LEFT_SQUEEZE_VALUE_ID);
        map.remove(&*RIGHT_SQUEEZE_VALUE_ID);
    }

    map
}

pub struct StreamingInteractionContext {
    pub action_set: xr::ActionSet,
    pub button_actions: HashMap<u64, ButtonAction>,
    pub left_hand_source: HandSource,
    pub right_hand_source: HandSource,
}

pub fn initialize_streaming_interaction(
    platform: Platform,
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
    xr_session: &xr::Session<xr::AnyGraphics>,
) -> StreamingInteractionContext {
    let action_set = xr_instance
        .create_action_set("alvr_input", "ALVR input", 0)
        .unwrap();

    let mut bindings = vec![];

    fn binding<'a, T: xr::ActionTy>(action: &'a xr::Action<T>, path: &str) -> xr::Binding<'a> {
        xr::Binding::new(action, action.instance().string_to_path(path).unwrap())
    }

    let bindings_info = get_button_bindings(platform);

    // Create actions:

    let mut button_actions = HashMap::new();
    for (id, info) in &bindings_info {
        let display_name = format!(
            "{}{}",
            info.name[0..1].to_uppercase(),
            info.name[1..].replace('_', " ")
        );

        let action = match info.binding_type {
            BindingType::Binary => ButtonAction::Binary(
                action_set
                    .create_action(&info.name, &display_name, &[])
                    .unwrap(),
            ),
            BindingType::Scalar => ButtonAction::Scalar(
                action_set
                    .create_action(&info.name, &display_name, &[])
                    .unwrap(),
            ),
        };
        button_actions.insert(*id, action);
    }

    let left_grip_action = action_set
        .create_action("left_hand_pose", "Left hand pose", &[])
        .unwrap();
    let right_grip_action = action_set
        .create_action("right_hand_pose", "Right hand pose", &[])
        .unwrap();

    let left_vibration_action = action_set
        .create_action("left_hand_vibration", "Left hand vibration", &[])
        .unwrap();
    let right_vibration_action = action_set
        .create_action("right_hand_vibration", "Right hand vibration", &[])
        .unwrap();

    // Create action bindings:

    for (id, action) in &button_actions {
        let path = &bindings_info.get(id).unwrap().binding_path;
        match action {
            ButtonAction::Binary(action) => {
                bindings.push(binding(action, path));
            }
            ButtonAction::Scalar(action) => {
                bindings.push(binding(action, path));
            }
        }
    }

    bindings.push(binding(
        &left_grip_action,
        "/user/hand/left/input/grip/pose",
    ));
    bindings.push(binding(
        &right_grip_action,
        "/user/hand/right/input/grip/pose",
    ));

    bindings.push(binding(
        &left_vibration_action,
        "/user/hand/left/output/haptic",
    ));
    bindings.push(binding(
        &right_vibration_action,
        "/user/hand/right/output/haptic",
    ));

    // Apply bindings:

    let controller_profile = match platform {
        Platform::Quest => QUEST_CONTROLLER_PROFILE,
        Platform::Pico => PICO_CONTROLLER_PROFILE,
        Platform::Vive => FOCUS3_CONTROLLER_PROFILE,
        Platform::Yvr => YVR_CONTROLLER_PROFILE,
        Platform::Other => QUEST_CONTROLLER_PROFILE,
    };

    xr_instance
        .suggest_interaction_profile_bindings(
            xr_instance.string_to_path(controller_profile).unwrap(),
            &bindings,
        )
        .unwrap();

    xr_session.attach_action_sets(&[&action_set]).unwrap();

    let left_grip_space = left_grip_action
        .create_space(xr_session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();
    let right_grip_space = right_grip_action
        .create_space(xr_session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();

    let (left_hand_tracker, right_hand_tracker) =
        if xr_instance.supports_hand_tracking(xr_system).unwrap() {
            (
                Some(xr_session.create_hand_tracker(xr::Hand::LEFT).unwrap()),
                Some(xr_session.create_hand_tracker(xr::Hand::RIGHT).unwrap()),
            )
        } else {
            (None, None)
        };

    let left_hand_source = HandSource {
        grip_action: left_grip_action,
        grip_space: left_grip_space,
        skeleton_tracker: left_hand_tracker,
        vibration_action: left_vibration_action,
    };

    let right_hand_source = HandSource {
        grip_action: right_grip_action,
        grip_space: right_grip_space,
        skeleton_tracker: right_hand_tracker,
        vibration_action: right_vibration_action,
    };

    StreamingInteractionContext {
        action_set,
        button_actions,
        left_hand_source,
        right_hand_source,
    }
}

pub fn get_hand_motion(
    session: &xr::Session<xr::AnyGraphics>,
    reference_space: &xr::Space,
    time: xr::Time,
    hand_source: &HandSource,
    last_position: &mut Vec3,
) -> StrResult<(Option<DeviceMotion>, Option<[Pose; 26]>)> {
    if hand_source
        .grip_action
        .is_active(session, xr::Path::NULL)
        .map_err(err!())?
    {
        let (location, velocity) = hand_source
            .grip_space
            .relate(reference_space, time)
            .map_err(err!())?;

        if location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_TRACKED)
        {
            *last_position = to_vec3(location.pose.position);
        }

        let hand_motion = DeviceMotion {
            pose: Pose {
                orientation: to_quat(location.pose.orientation),
                position: *last_position,
            },
            linear_velocity: to_vec3(velocity.linear_velocity),
            angular_velocity: to_vec3(velocity.angular_velocity),
        };

        return Ok((Some(hand_motion), None));
    }

    if let Some(tracker) = &hand_source.skeleton_tracker {
        // todo: support also velocities in the protocol
        if let Some((joint_locations, jont_velocities)) = reference_space
            .relate_hand_joints(tracker, time)
            .map_err(err!())?
        {
            let root_motion = DeviceMotion {
                pose: to_pose(joint_locations[0].pose),
                linear_velocity: to_vec3(jont_velocities[0].linear_velocity),
                angular_velocity: to_vec3(jont_velocities[0].angular_velocity),
            };

            let joints = joint_locations
                .iter()
                .map(|j| to_pose(j.pose))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            Ok((Some(root_motion), Some(joints)))
        } else {
            Ok((None, None))
        }
    } else {
        Ok((None, None))
    }
}

// todo: move emulation to server
fn emulate_missing_button_value(platform: Platform, click_action_id: u64, state: bool) {
    let scalar_value = ButtonValue::Scalar(if state { 1_f32 } else { 0_f32 });

    if platform == Platform::Yvr {
        if click_action_id == *LEFT_SQUEEZE_CLICK_ID {
            alvr_client_core::send_button(*LEFT_SQUEEZE_VALUE_ID, scalar_value);
        } else if click_action_id == *RIGHT_SQUEEZE_CLICK_ID {
            alvr_client_core::send_button(*RIGHT_SQUEEZE_VALUE_ID, scalar_value);
        }
    }
}

// todo: use hysteresis
// todo: move emulation to server
fn emulate_missing_button_click(platform: Platform, value_action_id: u64, state: f32) {
    let binary_value = ButtonValue::Binary(state > 0.5);

    if platform == Platform::Vive {
        if value_action_id == *LEFT_SQUEEZE_VALUE_ID {
            alvr_client_core::send_button(*LEFT_SQUEEZE_CLICK_ID, binary_value);
        } else if value_action_id == *LEFT_TRIGGER_VALUE_ID {
            alvr_client_core::send_button(*LEFT_TRIGGER_CLICK_ID, binary_value);
        } else if value_action_id == *RIGHT_SQUEEZE_VALUE_ID {
            alvr_client_core::send_button(*RIGHT_SQUEEZE_CLICK_ID, binary_value);
        } else if value_action_id == *RIGHT_TRIGGER_VALUE_ID {
            alvr_client_core::send_button(*RIGHT_TRIGGER_CLICK_ID, binary_value);
        }
    }
}

pub fn update_buttons(
    platform: Platform,
    xr_session: &xr::Session<xr::AnyGraphics>,
    button_actions: &HashMap<u64, ButtonAction>,
) -> StrResult {
    for (id, action) in button_actions {
        match action {
            ButtonAction::Binary(action) => {
                let state = action.state(xr_session, xr::Path::NULL).map_err(err!())?;

                if state.changed_since_last_sync {
                    alvr_client_core::send_button(*id, ButtonValue::Binary(state.current_state));

                    emulate_missing_button_value(platform, *id, state.current_state);
                }
            }
            ButtonAction::Scalar(action) => {
                let state = action.state(xr_session, xr::Path::NULL).map_err(err!())?;

                if state.changed_since_last_sync {
                    alvr_client_core::send_button(*id, ButtonValue::Scalar(state.current_state));

                    emulate_missing_button_click(platform, *id, state.current_state);
                }
            }
        }
    }

    Ok(())
}
