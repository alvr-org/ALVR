use crate::{
    extra_extensions::{
        BodyTrackerFB, EyeTrackerSocial, FaceTracker2FB, FacialTrackerHTC,
        BODY_JOINT_SET_FULL_BODY_META, FULL_BODY_JOINT_COUNT_META,
        FULL_BODY_JOINT_LEFT_FOOT_BALL_META, FULL_BODY_JOINT_LEFT_LOWER_LEG_META,
        FULL_BODY_JOINT_RIGHT_FOOT_BALL_META, FULL_BODY_JOINT_RIGHT_LOWER_LEG_META,
    },
    Platform, XrContext,
};
use alvr_common::{glam::Vec3, *};
use alvr_packets::{ButtonEntry, ButtonValue};
use alvr_session::{BodyTrackingSourcesConfig, FaceTrackingSourcesConfig};
use openxr as xr;
use std::collections::HashMap;
use xr::SpaceLocationFlags;

pub enum ButtonAction {
    Binary(xr::Action<bool>),
    Scalar(xr::Action<f32>),
}

pub struct HandInteraction {
    pub controllers_profile_id: u64,
    pub grip_action: xr::Action<xr::Posef>,
    pub grip_space: xr::Space,
    pub aim_action: xr::Action<xr::Posef>,
    pub aim_space: xr::Space,
    pub vibration_action: xr::Action<xr::Haptic>,
    pub skeleton_tracker: Option<xr::HandTracker>,
}

pub struct FaceSources {
    pub combined_eyes_source: Option<(xr::Action<xr::Posef>, xr::Space)>,
    pub eye_tracker_fb: Option<EyeTrackerSocial>,
    pub face_tracker_fb: Option<FaceTracker2FB>,
    pub eye_tracker_htc: Option<FacialTrackerHTC>,
    pub lip_tracker_htc: Option<FacialTrackerHTC>,
}

pub struct BodySources {
    pub body_tracker_fb: Option<(BodyTrackerFB, usize)>,
}

pub struct InteractionContext {
    pub action_set: xr::ActionSet,
    pub button_actions: HashMap<u64, ButtonAction>,
    pub hands_interaction: [HandInteraction; 2],
    pub face_sources: FaceSources,
    pub body_sources: BodySources,
}

pub fn initialize_interaction(
    xr_ctx: &XrContext,
    platform: Platform,
    face_tracking_sources: Option<FaceTrackingSourcesConfig>,
    body_tracking_sources: Option<BodyTrackingSourcesConfig>,
) -> InteractionContext {
    // todo: check which permissions are needed for htc
    #[cfg(target_os = "android")]
    if let Some(config) = &face_tracking_sources {
        if (config.combined_eye_gaze || config.eye_tracking_fb)
            && matches!(platform, Platform::Quest3 | Platform::QuestPro)
        {
            alvr_client_core::try_get_permission("com.oculus.permission.EYE_TRACKING")
        }
        if config.combined_eye_gaze && matches!(platform, Platform::Pico4 | Platform::PicoNeo3) {
            alvr_client_core::try_get_permission("com.picovr.permission.EYE_TRACKING")
        }
        if config.face_tracking_fb && matches!(platform, Platform::Quest3 | Platform::QuestPro) {
            alvr_client_core::try_get_permission("android.permission.RECORD_AUDIO");
            alvr_client_core::try_get_permission("com.oculus.permission.FACE_TRACKING")
        }
    }

    #[cfg(target_os = "android")]
    if let Some(config) = &body_tracking_sources {
        if (config.body_tracking_fb.enabled())
            && matches!(platform, Platform::Quest3 | Platform::QuestPro)
        {
            alvr_client_core::try_get_permission("com.oculus.permission.BODY_TRACKING")
        }
    }

    let action_set = xr_ctx
        .instance
        .create_action_set("alvr_interaction", "ALVR interaction", 0)
        .unwrap();

    let mut bindings = vec![];

    fn binding<'a, T: xr::ActionTy>(action: &'a xr::Action<T>, path: &str) -> xr::Binding<'a> {
        xr::Binding::new(action, action.instance().string_to_path(path).unwrap())
    }

    let controllers_profile_path = match platform {
        Platform::Quest1
        | Platform::Quest2
        | Platform::Quest3
        | Platform::QuestPro
        | Platform::QuestUnknown => QUEST_CONTROLLER_PROFILE_PATH, // todo: create new controller profile for quest pro and 3
        Platform::PicoNeo3 => PICO_NEO3_CONTROLLER_PROFILE_PATH,
        Platform::Pico4 => PICO4_CONTROLLER_PROFILE_PATH,
        Platform::Focus3 | Platform::XRElite | Platform::ViveUnknown => {
            FOCUS3_CONTROLLER_PROFILE_PATH
        }
        Platform::Yvr => YVR_CONTROLLER_PROFILE_PATH,
        _ => QUEST_CONTROLLER_PROFILE_PATH,
    };
    let controllers_profile_id = alvr_common::hash_string(controllers_profile_path);

    // Create actions:

    let mut button_actions = HashMap::new();
    for button_id in &CONTROLLER_PROFILE_INFO
        .get(&controllers_profile_id)
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
        .create_action("left_grip_pose", "Left grip pose", &[])
        .unwrap();
    let right_grip_action = action_set
        .create_action("right_grip_pose", "Right grip pose", &[])
        .unwrap();

    let left_aim_action = action_set
        .create_action("left_aim_pose", "Left aim pose", &[])
        .unwrap();
    let right_aim_action = action_set
        .create_action("right_aim_pose", "Right aim pose", &[])
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

    bindings.push(binding(&left_aim_action, "/user/hand/left/input/aim/pose"));
    bindings.push(binding(
        &right_aim_action,
        "/user/hand/right/input/aim/pose",
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
    xr_ctx
        .instance
        .suggest_interaction_profile_bindings(
            xr_ctx
                .instance
                .string_to_path(controllers_profile_path)
                .unwrap(),
            &bindings,
        )
        .unwrap();

    let combined_eyes_source = (face_tracking_sources
        .as_ref()
        .map(|s| s.combined_eye_gaze)
        .unwrap_or(false)
        && xr_ctx
            .extra_extensions
            .supports_eye_gaze_interaction(&xr_ctx.instance, xr_ctx.system))
    .then(|| {
        let action = action_set
            .create_action("combined_eye_gaze", "Combined eye gaze", &[])
            .unwrap();

        xr_ctx
            .instance
            .suggest_interaction_profile_bindings(
                xr_ctx
                    .instance
                    .string_to_path("/interaction_profiles/ext/eye_gaze_interaction")
                    .unwrap(),
                &[binding(&action, "/user/eyes_ext/input/gaze_ext/pose")],
            )
            .unwrap();

        let space = action
            .create_space(&xr_ctx.session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
            .unwrap();

        (action, space)
    });

    xr_ctx.session.attach_action_sets(&[&action_set]).unwrap();

    let left_grip_space = left_grip_action
        .create_space(&xr_ctx.session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();
    let right_grip_space = right_grip_action
        .create_space(&xr_ctx.session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();

    let left_aim_space = left_aim_action
        .create_space(&xr_ctx.session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();
    let right_aim_space = right_aim_action
        .create_space(&xr_ctx.session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
        .unwrap();

    let (left_hand_tracker, right_hand_tracker) =
        if xr_ctx.instance.exts().ext_hand_tracking.is_some()
            && xr_ctx
                .instance
                .supports_hand_tracking(xr_ctx.system)
                .unwrap()
        {
            (
                Some(xr_ctx.session.create_hand_tracker(xr::Hand::LEFT).unwrap()),
                Some(xr_ctx.session.create_hand_tracker(xr::Hand::RIGHT).unwrap()),
            )
        } else {
            (None, None)
        };

    let eye_tracker_fb = (face_tracking_sources
        .as_ref()
        .map(|s| s.eye_tracking_fb)
        .unwrap_or(false)
        && xr_ctx
            .extra_extensions
            .supports_social_eye_tracking(&xr_ctx.instance, xr_ctx.system))
    .then(|| {
        xr_ctx
            .extra_extensions
            .create_eye_tracker_social(xr_ctx.session.clone())
            .unwrap()
    });

    let face_tracker_fb = (face_tracking_sources
        .as_ref()
        .map(|s| s.face_tracking_fb)
        .unwrap_or(false)
        && xr_ctx
            .extra_extensions
            .supports_fb_visual_face_tracking(&xr_ctx.instance, xr_ctx.system)
        && xr_ctx
            .extra_extensions
            .supports_fb_audio_face_tracking(&xr_ctx.instance, xr_ctx.system))
    .then(|| {
        xr_ctx
            .extra_extensions
            .create_face_tracker2_fb(&xr_ctx.session, true, true)
            .unwrap()
    });

    let eye_tracker_htc = (face_tracking_sources
        .as_ref()
        .map(|s| s.eye_expressions_htc)
        .unwrap_or(false)
        && xr_ctx
            .extra_extensions
            .supports_htc_eye_facial_tracking(&xr_ctx.instance, xr_ctx.system))
    .then(|| {
        xr_ctx
            .extra_extensions
            .create_facial_tracker_htc(&xr_ctx.session, xr::FacialTrackingTypeHTC::EYE_DEFAULT)
            .unwrap()
    });

    let lip_tracker_htc = (face_tracking_sources
        .map(|s| s.lip_expressions_htc)
        .unwrap_or(false)
        && xr_ctx
            .extra_extensions
            .supports_htc_lip_facial_tracking(&xr_ctx.instance, xr_ctx.system))
    .then(|| {
        xr_ctx
            .extra_extensions
            .create_facial_tracker_htc(&xr_ctx.session, xr::FacialTrackingTypeHTC::LIP_DEFAULT)
            .unwrap()
    });

    let body_tracker_fb = if let Some(body_tracking_fb) =
        body_tracking_sources.and_then(|s| s.body_tracking_fb.into_option())
    {
        if body_tracking_fb.full_body
            && xr_ctx
                .extra_extensions
                .supports_full_body_tracking_meta(&xr_ctx.instance, xr_ctx.system)
        {
            let tracker = xr_ctx
                .extra_extensions
                .create_body_tracker_fb(&xr_ctx.session, *BODY_JOINT_SET_FULL_BODY_META)
                .unwrap();

            Some((tracker, FULL_BODY_JOINT_COUNT_META))
        } else if xr_ctx
            .extra_extensions
            .supports_body_tracking_fb(&xr_ctx.instance, xr_ctx.system)
        {
            let tracker = xr_ctx
                .extra_extensions
                .create_body_tracker_fb(&xr_ctx.session, xr::BodyJointSetFB::DEFAULT)
                .unwrap();

            Some((tracker, xr::BodyJointFB::COUNT.into_raw() as usize))
        } else {
            None
        }
    } else {
        None
    };

    InteractionContext {
        action_set,
        button_actions,
        hands_interaction: [
            HandInteraction {
                controllers_profile_id,
                grip_action: left_grip_action,
                grip_space: left_grip_space,
                aim_action: left_aim_action,
                aim_space: left_aim_space,
                vibration_action: left_vibration_action,
                skeleton_tracker: left_hand_tracker,
            },
            HandInteraction {
                controllers_profile_id,
                grip_action: right_grip_action,
                grip_space: right_grip_space,
                aim_action: right_aim_action,
                aim_space: right_aim_space,
                vibration_action: right_vibration_action,
                skeleton_tracker: right_hand_tracker,
            },
        ],
        face_sources: FaceSources {
            combined_eyes_source,
            eye_tracker_fb,
            face_tracker_fb,
            eye_tracker_htc,
            lip_tracker_htc,
        },
        body_sources: BodySources { body_tracker_fb },
    }
}

pub fn get_reference_space(
    xr_session: &xr::Session<xr::OpenGlEs>,
    ty: xr::ReferenceSpaceType,
) -> xr::Space {
    xr_session
        .create_reference_space(ty, xr::Posef::IDENTITY)
        .unwrap()
}

pub fn get_hand_data(
    xr_session: &xr::Session<xr::OpenGlEs>,
    reference_space: &xr::Space,
    time: xr::Time,
    hand_source: &HandInteraction,
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
                *last_position = crate::from_xr_vec3(joint_locations[0].pose.position);
            }

            let root_motion = DeviceMotion {
                pose: Pose {
                    orientation: crate::from_xr_quat(joint_locations[0].pose.orientation),
                    position: *last_position,
                },
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            };

            let joints = joint_locations
                .iter()
                .map(|j| crate::from_xr_pose(j.pose))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            return (Some(root_motion), Some(joints));
        }
    }

    if !hand_source
        .grip_action
        .is_active(xr_session, xr::Path::NULL)
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
        *last_position = crate::from_xr_vec3(location.pose.position);
    }

    let hand_motion = DeviceMotion {
        pose: Pose {
            orientation: crate::from_xr_quat(location.pose.orientation),
            position: *last_position,
        },
        linear_velocity: crate::from_xr_vec3(velocity.linear_velocity),
        angular_velocity: crate::from_xr_vec3(velocity.angular_velocity),
    };

    (Some(hand_motion), None)
}

pub fn update_buttons(
    xr_session: &xr::Session<xr::OpenGlEs>,
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

pub fn get_eye_gazes(
    xr_session: &xr::Session<xr::OpenGlEs>,
    sources: &FaceSources,
    reference_space: &xr::Space,
    time: xr::Time,
) -> [Option<Pose>; 2] {
    'fb_eyes: {
        let Some(tracker) = &sources.eye_tracker_fb else {
            break 'fb_eyes;
        };

        if let Ok(gazes) = tracker.get_eye_gazes(reference_space, time) {
            return [
                gazes[0].map(crate::from_xr_pose),
                gazes[1].map(crate::from_xr_pose),
            ];
        }
    };

    let Some((eyes_action, eyes_space)) = &sources.combined_eyes_source else {
        return [None, None];
    };
    if !eyes_action
        .is_active(xr_session, xr::Path::NULL)
        .unwrap_or(false)
    {
        return [None, None];
    }

    if let Ok(location) = eyes_space.locate(reference_space, time) {
        [
            location
                .location_flags
                .contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED)
                .then(|| crate::from_xr_pose(location.pose)),
            None,
        ]
    } else {
        [None, None]
    }
}

pub fn get_fb_face_expression(context: &FaceSources, time: xr::Time) -> Option<Vec<f32>> {
    context
        .face_tracker_fb
        .as_ref()
        .and_then(|t| t.get_face_expression_weights(time).ok().flatten())
        .map(|weights| weights.into_iter().collect())
}

pub fn get_htc_eye_expression(context: &FaceSources) -> Option<Vec<f32>> {
    context
        .eye_tracker_htc
        .as_ref()
        .and_then(|t| t.get_facial_expressions().ok().flatten())
        .map(|weights| weights.into_iter().collect())
}

pub fn get_htc_lip_expression(context: &FaceSources) -> Option<Vec<f32>> {
    context
        .lip_tracker_htc
        .as_ref()
        .and_then(|t| t.get_facial_expressions().ok().flatten())
        .map(|weights| weights.into_iter().collect())
}

pub fn get_fb_body_skeleton(
    reference_space: &xr::Space,
    time: xr::Time,
    body_tracker: &BodyTrackerFB,
    joint_count: usize,
) -> Option<Vec<Option<Pose>>> {
    body_tracker
        .locate_body_joints(time, reference_space, joint_count)
        .ok()
        .flatten()
        .map(|joints| {
            let valid_flags: SpaceLocationFlags =
                SpaceLocationFlags::ORIENTATION_VALID | SpaceLocationFlags::POSITION_VALID;

            joints
                .iter()
                .map(|joint| {
                    joint
                        .location_flags
                        .contains(valid_flags)
                        .then(|| crate::from_xr_pose(joint.pose))
                })
                .collect()
        })
}

pub fn get_fb_body_tracking_points(
    reference_space: &xr::Space,
    time: xr::Time,
    body_tracker: &BodyTrackerFB,
    joint_count: usize,
) -> Vec<(u64, DeviceMotion)> {
    if let Some(joint_locations) = body_tracker
        .locate_body_joints(time, reference_space, joint_count)
        .ok()
        .flatten()
    {
        let valid_flags: SpaceLocationFlags =
            SpaceLocationFlags::ORIENTATION_VALID | SpaceLocationFlags::POSITION_VALID;

        let mut joints = Vec::<(u64, DeviceMotion)>::with_capacity(8);

        if let Some(joint) = joint_locations.get(xr::BodyJointFB::CHEST.into_raw() as usize) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_CHEST_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) = joint_locations.get(xr::BodyJointFB::HIPS.into_raw() as usize) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_HIPS_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) =
            joint_locations.get(xr::BodyJointFB::LEFT_ARM_LOWER.into_raw() as usize)
        {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_LEFT_ELBOW_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) =
            joint_locations.get(xr::BodyJointFB::RIGHT_ARM_LOWER.into_raw() as usize)
        {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_RIGHT_ELBOW_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) = joint_locations.get(FULL_BODY_JOINT_LEFT_LOWER_LEG_META) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_LEFT_KNEE_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) = joint_locations.get(FULL_BODY_JOINT_LEFT_FOOT_BALL_META) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_LEFT_FOOT_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) = joint_locations.get(FULL_BODY_JOINT_RIGHT_LOWER_LEG_META) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_RIGHT_KNEE_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        if let Some(joint) = joint_locations.get(FULL_BODY_JOINT_RIGHT_FOOT_BALL_META) {
            if joint.location_flags.contains(valid_flags) {
                joints.push((
                    *BODY_RIGHT_FOOT_ID,
                    DeviceMotion {
                        pose: crate::from_xr_pose(joint.pose),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ))
            }
        }

        return joints;
    }

    Vec::new()
}
