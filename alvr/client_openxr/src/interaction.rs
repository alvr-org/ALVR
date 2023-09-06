use crate::{to_pose, to_quat, to_vec3, Platform};
use alvr_common::{glam::Vec3, *};
use alvr_packets::{ButtonEntry, ButtonValue};
use openxr as xr;
use std::collections::HashMap;

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

pub struct HandsInteractionContext {
    pub interaction_profile_id: u64,
    pub action_set: xr::ActionSet,
    pub button_actions: HashMap<u64, ButtonAction>,
    pub hand_sources: [HandSource; 2],
}

pub fn initialize_hands_interaction(
    platform: Platform,
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
    xr_session: &xr::Session<xr::AnyGraphics>,
) -> HandsInteractionContext {
    let action_set = xr_instance
        .create_action_set("alvr_input", "ALVR input", 0)
        .unwrap();

    let mut bindings = vec![];

    fn binding<'a, T: xr::ActionTy>(action: &'a xr::Action<T>, path: &str) -> xr::Binding<'a> {
        xr::Binding::new(action, action.instance().string_to_path(path).unwrap())
    }

    let interaction_profile_path = match platform {
        Platform::Quest => QUEST_CONTROLLER_PROFILE_PATH,
        Platform::PicoNeo3 => PICO_NEO3_CONTROLLER_PROFILE_PATH,
        Platform::Pico4 => PICO4_CONTROLLER_PROFILE_PATH,
        Platform::Focus3 => FOCUS3_CONTROLLER_PROFILE_PATH,
        Platform::Yvr => YVR_CONTROLLER_PROFILE_PATH,
        _ => QUEST_CONTROLLER_PROFILE_PATH,
    };
    let interaction_profile_id = alvr_common::hash_string(interaction_profile_path);

    // Create actions:

    let mut button_actions = HashMap::new();
    for button_id in &CONTROLLER_PROFILE_INFO
        .get(&interaction_profile_id)
        .unwrap()
        .button_set
    {
        let info = BUTTON_INFO.get(button_id).unwrap();

        let name = info.path[1..].replace('/', "_");
        let display_name = format!(
            "{}{}",
            name[0..1].to_uppercase(),
            name[1..].replace('_', " ")
        );

        let action = match info.button_type {
            ButtonType::Binary => {
                ButtonAction::Binary(action_set.create_action(&name, &display_name, &[]).unwrap())
            }
            ButtonType::Scalar => {
                ButtonAction::Scalar(action_set.create_action(&name, &display_name, &[]).unwrap())
            }
        };
        button_actions.insert(*button_id, action);
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
        let path = &BUTTON_INFO.get(id).unwrap().path;
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
    xr_instance
        .suggest_interaction_profile_bindings(
            xr_instance
                .string_to_path(interaction_profile_path)
                .unwrap(),
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

    let (left_hand_tracker, right_hand_tracker) = if xr_instance.exts().ext_hand_tracking.is_some()
        && xr_instance.supports_hand_tracking(xr_system).unwrap()
    {
        (
            Some(xr_session.create_hand_tracker(xr::Hand::LEFT).unwrap()),
            Some(xr_session.create_hand_tracker(xr::Hand::RIGHT).unwrap()),
        )
    } else {
        (None, None)
    };

    let hand_sources = [
        HandSource {
            grip_action: left_grip_action,
            grip_space: left_grip_space,
            skeleton_tracker: left_hand_tracker,
            vibration_action: left_vibration_action,
        },
        HandSource {
            grip_action: right_grip_action,
            grip_space: right_grip_space,
            skeleton_tracker: right_hand_tracker,
            vibration_action: right_vibration_action,
        },
    ];

    HandsInteractionContext {
        interaction_profile_id,
        action_set,
        button_actions,
        hand_sources,
    }
}

pub fn get_hand_motion(
    session: &xr::Session<xr::AnyGraphics>,
    reference_space: &xr::Space,
    time: xr::Time,
    hand_source: &HandSource,
    last_position: &mut Vec3,
) -> (Option<DeviceMotion>, Option<[Pose; 26]>) {
    if let Some(tracker) = &hand_source.skeleton_tracker {
        if let Some(joint_locations) = reference_space
            .locate_hand_joints(tracker, time)
            .ok()
            .flatten()
        {
            if joint_locations[0]
                .location_flags
                .contains(xr::SpaceLocationFlags::POSITION_VALID)
            {
                *last_position = to_vec3(joint_locations[0].pose.position);
            }

            let root_motion = DeviceMotion {
                pose: Pose {
                    orientation: to_quat(joint_locations[0].pose.orientation),
                    position: *last_position,
                },
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            };

            let joints = joint_locations
                .iter()
                .map(|j| to_pose(j.pose))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            return (Some(root_motion), Some(joints));
        }
    }

    if !hand_source
        .grip_action
        .is_active(session, xr::Path::NULL)
        .unwrap_or(false)
    {
        return (None, None);
    }

    let Ok((location, velocity)) = hand_source.grip_space.relate(reference_space, time) else {
        return (None, None);
    };

    if !location
        .location_flags
        .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
    {
        return (None, None);
    }

    if location
        .location_flags
        .contains(xr::SpaceLocationFlags::POSITION_VALID)
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

    (Some(hand_motion), None)
}

pub fn update_buttons(
    xr_session: &xr::Session<xr::AnyGraphics>,
    button_actions: &HashMap<u64, ButtonAction>,
) -> Vec<ButtonEntry> {
    let mut button_entries = Vec::with_capacity(2);
    for (id, action) in button_actions {
        match action {
            ButtonAction::Binary(action) => {
                let Ok(state) = action.state(xr_session, xr::Path::NULL) else {
                    continue;
                };

                if state.changed_since_last_sync {
                    button_entries.push(ButtonEntry {
                        path_id: *id,
                        value: ButtonValue::Binary(state.current_state),
                    });
                }
            }
            ButtonAction::Scalar(action) => {
                let Ok(state) = action.state(xr_session, xr::Path::NULL) else {
                    continue;
                };

                if state.changed_since_last_sync {
                    button_entries.push(ButtonEntry {
                        path_id: *id,
                        value: ButtonValue::Scalar(state.current_state),
                    });
                }
            }
        }
    }

    button_entries
}

pub struct FaceInputContext {
    pub eye_tracker_fb: Option<xr::EyeTrackerSocial>,
    pub face_tracker_fb: Option<xr::FaceTrackerFB>,
    pub eye_tracker_htc: Option<xr::FacialTrackerHTC>,
    pub lip_tracker_htc: Option<xr::FacialTrackerHTC>,
}

pub fn initialize_face_input<G>(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
    xr_session: &xr::Session<G>,
    eye_tracking_fb: bool,
    face_tracking_fb: bool,
    eye_expressions_htc: bool,
    lip_expressions_htc: bool,
) -> FaceInputContext {
    let eye_tracker_fb = (eye_tracking_fb
        && xr_instance.exts().fb_eye_tracking_social.is_some()
        && xr_instance.supports_social_eye_tracking(xr_system).unwrap())
    .then(|| xr_session.create_eye_tracker_social().unwrap());

    let face_tracker_fb = (face_tracking_fb
        && xr_instance.exts().fb_face_tracking.is_some()
        && xr_instance.supports_fb_face_tracking(xr_system).unwrap())
    .then(|| xr_session.create_face_tracker_fb().unwrap());

    let eye_tracker_htc = (eye_expressions_htc
        && xr_instance.exts().htc_facial_tracking.is_some()
        && xr_instance
            .supports_htc_eye_facial_tracking(xr_system)
            .unwrap())
    .then(|| {
        xr_session
            .create_facial_tracker_htc(xr::FacialTrackingTypeHTC::EYE_DEFAULT)
            .unwrap()
    });

    let lip_tracker_htc = (lip_expressions_htc
        && xr_instance.exts().htc_facial_tracking.is_some()
        && xr_instance
            .supports_htc_lip_facial_tracking(xr_system)
            .unwrap())
    .then(|| {
        xr_session
            .create_facial_tracker_htc(xr::FacialTrackingTypeHTC::LIP_DEFAULT)
            .unwrap()
    });

    FaceInputContext {
        eye_tracker_fb,
        face_tracker_fb,
        eye_tracker_htc,
        lip_tracker_htc,
    }
}

pub fn get_eye_gazes(
    context: &FaceInputContext,
    reference_space: &xr::Space,
    time: xr::Time,
) -> [Option<Pose>; 2] {
    let Some(tracker) = &context.eye_tracker_fb else {
        return [None, None];
    };

    if let Ok(gazes) = tracker.get_eye_gazes(reference_space, time) {
        [
            gazes.gaze[0].as_ref().map(|g| to_pose(g.pose)),
            gazes.gaze[1].as_ref().map(|g| to_pose(g.pose)),
        ]
    } else {
        [None, None]
    }
}

pub fn get_fb_face_expression(context: &FaceInputContext, time: xr::Time) -> Option<Vec<f32>> {
    context
        .face_tracker_fb
        .as_ref()
        .and_then(|t| t.get_face_expression_weights(time).ok().flatten())
        .map(|w| w.weights.into_iter().collect())
}

pub fn get_htc_eye_expression(context: &FaceInputContext) -> Option<Vec<f32>> {
    context
        .eye_tracker_htc
        .as_ref()
        .and_then(|t| t.get_facial_expressions().ok().flatten())
        .map(|w| w.weights.into_iter().collect())
}

pub fn get_htc_lip_expression(context: &FaceInputContext) -> Option<Vec<f32>> {
    context
        .lip_tracker_htc
        .as_ref()
        .and_then(|t| t.get_facial_expressions().ok().flatten())
        .map(|w| w.weights.into_iter().collect())
}
