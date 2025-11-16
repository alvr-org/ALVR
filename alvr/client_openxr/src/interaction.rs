use crate::{
    Platform,
    extra_extensions::{
        self, BODY_JOINT_SET_FULL_BODY_META, BodyJointSetBD, BodyTrackerBD, BodyTrackerFB,
        EyeTrackerSocial, FULL_BODY_JOINT_COUNT_META, FaceTracker2FB, FaceTrackerPico,
        FacialTrackerHTC, MotionTrackerBD, MultimodalMeta,
    },
};
use alvr_common::{
    glam::{Quat, Vec3},
    *,
};
use alvr_graphics::HandData;
use alvr_packets::{ButtonEntry, ButtonValue, FaceData, FaceExpressions, StreamConfig};
use alvr_session::{BodyTrackingBDConfig, BodyTrackingSourcesConfig, FaceTrackingSourcesConfig};
use openxr as xr;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use xr::SpaceLocationFlags;

const IPD_CHANGE_EPS: f32 = 0.001;

// Most OpenXR runtime, including Meta's one, do not follow perfectly the specification regarding
// controller pose. The Z axis should point down through the center of the controller grip, the X
// axis should go out perpendicular from the palm, and the position should be aligned roughtly with
// the center of the palm.
// https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html
// Note: right controller offsets are calculated from left controller offsets by mirroring along the
// Y-Z plane.
fn get_controller_offset(platform: Platform, is_right_hand: bool) -> Pose {
    const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;

    let left_offset = match platform {
        Platform::Quest1 => Pose {
            position: Vec3::new(-0.013, -0.005, 0.0),
            orientation: Quat::from_rotation_x(-20.0 * DEG_TO_RAD),
        },
        // todo: check Quest 2
        p if p.is_quest() => Pose {
            position: Vec3::new(-0.005, -0.005, 0.00),
            orientation: Quat::from_rotation_x(-15.0 * DEG_TO_RAD),
        },
        Platform::PicoNeo3 => Pose {
            position: Vec3::new(-0.013, -0.035, 0.0),
            orientation: Quat::IDENTITY,
        },
        // todo: check (base) Pico 4
        p if p.is_pico() => Pose {
            position: Vec3::new(-0.01, -0.035, 0.0),
            orientation: Quat::from_rotation_y(6.0 * DEG_TO_RAD)
                * Quat::from_rotation_x(-6.0 * DEG_TO_RAD),
        },
        p if p.is_vive() => Pose {
            position: Vec3::new(0.0, 0.0, -0.02),
            orientation: Quat::IDENTITY,
        },
        Platform::SamsungGalaxyXR => Pose {
            position: Vec3::new(0.0, 0.0, 0.055),
            orientation: Quat::IDENTITY,
        },
        _ => Pose::IDENTITY,
    };

    if is_right_hand {
        let p = left_offset.position;
        let q = left_offset.orientation;

        Pose {
            position: Vec3::new(-p[0], p[1], p[2]),
            orientation: Quat::from_xyzw(-q.x, q.y, q.z, -q.w),
        }
    } else {
        left_offset
    }
}

fn check_ext_object<T>(name: &str, result: xr::Result<T>) -> Option<T> {
    match result {
        Ok(obj) => Some(obj),
        Err(xr::sys::Result::ERROR_FEATURE_UNSUPPORTED) => {
            warn!("Cannot create unsupported {name}");
            None
        }
        Err(xr::sys::Result::ERROR_EXTENSION_NOT_PRESENT) => None,
        Err(e) => {
            warn!("Failed to create {name}: {e}");
            None
        }
    }
}

pub enum ButtonAction {
    Binary(xr::Action<bool>),
    Scalar(xr::Action<f32>),
}

pub struct HandInteraction {
    pub controllers_profile_id: u64,
    pub input_ids: HashSet<u64>,
    pub pose_offset: Pose,

    pub grip_action: xr::Action<xr::Posef>,
    pub grip_space: xr::Space,

    #[expect(dead_code)]
    pub aim_action: xr::Action<xr::Posef>,
    #[expect(dead_code)]
    pub aim_space: xr::Space,

    pub detached_grip_action: Option<xr::Action<xr::Posef>>,
    pub detached_grip_space: Option<xr::Space>,

    pub vibration_action: xr::Action<xr::Haptic>,
    pub skeleton_tracker: Option<xr::HandTracker>,
}

pub enum FaceExpressionsTracker {
    Fb(FaceTracker2FB),
    Pico(FaceTrackerPico),
    Htc {
        eye: Option<FacialTrackerHTC>,
        lip: Option<FacialTrackerHTC>,
    },
}

pub struct FaceSources {
    eyes_combined: Option<(xr::Action<xr::Posef>, xr::Space)>,
    eyes_social: Option<EyeTrackerSocial>,
    face_expressions_tracker: Option<FaceExpressionsTracker>,
}

pub enum BodyTracker {
    Fb {
        tracker: BodyTrackerFB,
        joint_count: usize,
    },
    BodyBD(BodyTrackerBD),
    MotionBD(MotionTrackerBD),
}

#[derive(Clone)]
pub struct InteractionSourcesConfig {
    pub face_tracking: Option<FaceTrackingSourcesConfig>,
    pub body_tracking: Option<BodyTrackingSourcesConfig>,
    pub prefers_multimodal_input: bool,
}

impl InteractionSourcesConfig {
    pub fn new(config: &StreamConfig) -> Self {
        Self {
            face_tracking: config
                .settings
                .headset
                .face_tracking
                .as_option()
                .map(|c| c.sources.clone()),
            body_tracking: config
                .settings
                .headset
                .body_tracking
                .as_option()
                .map(|c| c.sources.clone()),
            prefers_multimodal_input: config
                .settings
                .headset
                .multimodal_tracking
                .as_option()
                .is_some_and(|c| c.enabled),
        }
    }
}

pub struct InteractionContext {
    xr_session: xr::Session<xr::OpenGlEs>,
    xr_system: xr::SystemId,
    extra_extensions: Vec<String>,
    platform: Platform,
    pub action_set: xr::ActionSet,
    pub button_actions: HashMap<u64, ButtonAction>,
    pub hands_interaction: [HandInteraction; 2],
    multimodal_handle: Option<MultimodalMeta>,
    pub multimodal_hands_enabled: bool,
    pub face_sources: FaceSources,
    pub body_source: Option<BodyTracker>,
}

impl InteractionContext {
    pub fn new(
        xr_session: xr::Session<xr::OpenGlEs>,
        extra_extensions: Vec<String>,
        xr_system: xr::SystemId,
        platform: Platform,
    ) -> Self {
        let xr_instance = xr_session.instance();

        let action_set = xr_instance
            .create_action_set("alvr_interaction", "ALVR interaction", 0)
            .unwrap();

        let mut bindings = vec![];

        fn binding<'a, T: xr::ActionTy>(action: &'a xr::Action<T>, path: &str) -> xr::Binding<'a> {
            xr::Binding::new(action, action.instance().string_to_path(path).unwrap())
        }

        let controllers_profile_path = match platform {
            p if p.is_quest() => QUEST_CONTROLLER_PROFILE_PATH, // todo: create new controller profile for quest pro and 3
            Platform::PicoG3 => PICO_G3_CONTROLLER_PROFILE_PATH,
            Platform::PicoNeo3 => PICO_NEO3_CONTROLLER_PROFILE_PATH,
            Platform::Pico4Ultra => PICO4S_CONTROLLER_PROFILE_PATH,
            Platform::Pico4 | Platform::Pico4Pro | Platform::Pico4Enterprise => {
                PICO4_CONTROLLER_PROFILE_PATH
            }
            p if p.is_pico() => PICO4S_CONTROLLER_PROFILE_PATH,
            p if p.is_vive() => FOCUS3_CONTROLLER_PROFILE_PATH,
            p if p.is_yvr() => YVR_CONTROLLER_PROFILE_PATH,
            _ => QUEST_CONTROLLER_PROFILE_PATH,
        };
        let controllers_profile_id = alvr_common::hash_string(controllers_profile_path);

        // Create actions:

        let mut button_actions = HashMap::new();
        let button_set = CONTROLLER_PROFILE_INFO
            .get(&controllers_profile_id)
            .unwrap()
            .button_set
            .clone();
        for button_id in &button_set {
            let info = BUTTON_INFO.get(button_id).unwrap();

            let name = info.path[1..].replace('/', "_");
            let display_name = format!(
                "{}{}",
                name[0..1].to_uppercase(),
                name[1..].replace('_', " ")
            );

            let action = match info.button_type {
                ButtonType::Binary => ButtonAction::Binary(
                    action_set.create_action(&name, &display_name, &[]).unwrap(),
                ),
                ButtonType::Scalar => ButtonAction::Scalar(
                    action_set.create_action(&name, &display_name, &[]).unwrap(),
                ),
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

        let multimodal_handle = check_ext_object(
            "MultimodalMeta",
            MultimodalMeta::new(xr_session.clone(), &extra_extensions, xr_system),
        );

        let mut left_detached_grip_action = None;
        let mut right_detached_grip_action = None;
        if multimodal_handle.is_some() {
            // Note: when multimodal input is enabled, both controllers and hands will always be
            // active. Held controllers and detached controllers are sent to the server as separate
            // devices.
            let left_detached_grip_action = left_detached_grip_action.insert(
                action_set
                    .create_action::<xr::Posef>(
                        "left_detached_grip_pose",
                        "Left detached grip pose",
                        &[],
                    )
                    .unwrap(),
            );
            let right_detached_grip_action = right_detached_grip_action.insert(
                action_set
                    .create_action::<xr::Posef>(
                        "right_detached_grip_pose",
                        "Right detached grip pose",
                        &[],
                    )
                    .unwrap(),
            );

            bindings.push(binding(
                left_detached_grip_action,
                "/user/detached_controller_meta/left/input/grip/pose",
            ));
            bindings.push(binding(
                right_detached_grip_action,
                "/user/detached_controller_meta/right/input/grip/pose",
            ));
        }

        // Apply bindings:
        xr_instance
            .suggest_interaction_profile_bindings(
                xr_instance
                    .string_to_path(controllers_profile_path)
                    .unwrap(),
                &bindings,
            )
            .unwrap();

        let left_grip_space = left_grip_action
            .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
            .unwrap();
        let right_grip_space = right_grip_action
            .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
            .unwrap();

        let left_aim_space = left_aim_action
            .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
            .unwrap();
        let right_aim_space = right_aim_action
            .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
            .unwrap();

        let left_detached_grip_space = left_detached_grip_action.as_ref().map(|action| {
            action
                .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap()
        });
        let right_detached_grip_space = right_detached_grip_action.as_ref().map(|action| {
            action
                .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap()
        });

        let left_hand_tracker = check_ext_object(
            "HandTracker (left)",
            xr_session.create_hand_tracker(xr::Hand::LEFT),
        );
        let right_hand_tracker = check_ext_object(
            "HandTracker (right)",
            xr_session.create_hand_tracker(xr::Hand::RIGHT),
        );

        let eyes_combined =
            if extra_extensions::supports_eye_gaze_interaction(&xr_session, xr_system) {
                if matches!(platform, Platform::QuestPro) {
                    #[cfg(target_os = "android")]
                    alvr_system_info::try_get_permission("com.oculus.permission.EYE_TRACKING");
                } else if matches!(
                    platform,
                    Platform::PicoNeo3 | Platform::Pico4Pro | Platform::Pico4Enterprise
                ) {
                    #[cfg(target_os = "android")]
                    alvr_system_info::try_get_permission("com.picovr.permission.EYE_TRACKING");
                }

                let action = action_set
                    .create_action("combined_eye_gaze", "Combined eye gaze", &[])
                    .unwrap();

                let res = xr_instance.suggest_interaction_profile_bindings(
                    xr_instance
                        .string_to_path("/interaction_profiles/ext/eye_gaze_interaction")
                        .unwrap(),
                    &[binding(&action, "/user/eyes_ext/input/gaze_ext/pose")],
                );
                if res.is_err() {
                    warn!("Failed to register combined eye gaze input: {res:?}");
                }

                let space = action
                    .create_space(&xr_session, xr::Path::NULL, xr::Posef::IDENTITY)
                    .unwrap();

                Some((action, space))
            } else {
                None
            };

        // Note: HTC facial tracking can only be created at startup before xrBeginSession. We don't
        // know the reason.
        let face_expressions_tracker = if platform.is_vive() {
            let eye = check_ext_object(
                "FacialTrackerHTC (eyes)",
                FacialTrackerHTC::new(
                    xr_session.clone(),
                    xr_system,
                    xr::FacialTrackingTypeHTC::EYE_DEFAULT,
                ),
            );
            let lip = check_ext_object(
                "FacialTrackerHTC (lips)",
                FacialTrackerHTC::new(
                    xr_session.clone(),
                    xr_system,
                    xr::FacialTrackingTypeHTC::LIP_DEFAULT,
                ),
            );
            Some(FaceExpressionsTracker::Htc { eye, lip })
        } else {
            None
        };

        xr_session.attach_action_sets(&[&action_set]).unwrap();

        Self {
            xr_session,
            xr_system,
            extra_extensions,
            platform,
            action_set,
            button_actions,
            hands_interaction: [
                HandInteraction {
                    controllers_profile_id,
                    input_ids: button_set.clone(),
                    pose_offset: get_controller_offset(platform, false),
                    grip_action: left_grip_action,
                    grip_space: left_grip_space,
                    aim_action: left_aim_action,
                    aim_space: left_aim_space,
                    detached_grip_action: left_detached_grip_action,
                    detached_grip_space: left_detached_grip_space,
                    vibration_action: left_vibration_action,
                    skeleton_tracker: left_hand_tracker,
                },
                HandInteraction {
                    controllers_profile_id,
                    input_ids: button_set,
                    pose_offset: get_controller_offset(platform, true),
                    grip_action: right_grip_action,
                    grip_space: right_grip_space,
                    aim_action: right_aim_action,
                    aim_space: right_aim_space,
                    detached_grip_action: right_detached_grip_action,
                    detached_grip_space: right_detached_grip_space,
                    vibration_action: right_vibration_action,
                    skeleton_tracker: right_hand_tracker,
                },
            ],
            multimodal_handle,
            multimodal_hands_enabled: false,
            face_sources: FaceSources {
                eyes_combined,
                eyes_social: None,
                face_expressions_tracker,
            },
            body_source: None,
        }
    }

    pub fn select_sources(&mut self, config: &InteractionSourcesConfig) {
        // First of all, disable/delete all sources. This ensures there are no conflicts
        if let Some(handle) = &mut self.multimodal_handle {
            handle.pause().ok();
        }

        if let Some(FaceExpressionsTracker::Pico(tracker)) =
            &self.face_sources.face_expressions_tracker
        {
            tracker.stop_face_tracking().ok();
        }

        self.multimodal_hands_enabled = false;
        self.face_sources.eyes_social = None;

        // HTC trackers must not be destroyed or the app will crash
        if !matches!(
            self.face_sources.face_expressions_tracker,
            Some(FaceExpressionsTracker::Htc { .. })
        ) {
            self.face_sources.face_expressions_tracker = None;
        }

        self.body_source = None;

        if let Some(config) = &config.face_tracking {
            if matches!(self.platform, Platform::QuestPro)
                && matches!(config, FaceTrackingSourcesConfig::PreferFullFaceTracking)
            {
                #[cfg(target_os = "android")]
                {
                    alvr_system_info::try_get_permission("android.permission.RECORD_AUDIO");
                    alvr_system_info::try_get_permission("com.oculus.permission.FACE_TRACKING")
                }
            }

            if matches!(
                self.platform,
                Platform::PicoNeo3 | Platform::Pico4Pro | Platform::Pico4Enterprise
            ) && matches!(config, FaceTrackingSourcesConfig::PreferFullFaceTracking)
                && extra_extensions::supports_eye_gaze_interaction(&self.xr_session, self.xr_system)
            {
                #[cfg(target_os = "android")]
                {
                    alvr_system_info::try_get_permission("android.permission.RECORD_AUDIO");
                    alvr_system_info::try_get_permission("com.picovr.permission.FACE_TRACKING")
                }
            }
        }

        if config.body_tracking.is_some()
            && self.platform.is_quest()
            && self.platform != Platform::Quest1
        {
            #[cfg(target_os = "android")]
            alvr_system_info::try_get_permission("com.oculus.permission.BODY_TRACKING")
        }

        // Note: We cannot enable multimodal if fb body tracking is active. It would result in a
        // ERROR_RUNTIME_FAILURE crash.
        if config.prefers_multimodal_input
            && config.body_tracking.is_none()
            && let Some(handle) = &mut self.multimodal_handle
            && handle.resume().is_ok()
        {
            self.multimodal_hands_enabled = true;
        }

        if let Some(config) = &config.face_tracking {
            // Note: this is actually used by multiple vendors
            self.face_sources.eyes_social =
                check_ext_object("EyeTrackerSocial", EyeTrackerSocial::new(&self.xr_session));

            if matches!(config, FaceTrackingSourcesConfig::PreferFullFaceTracking) {
                if let Some(tracker) = check_ext_object(
                    "FaceTracker2FB",
                    FaceTracker2FB::new(self.xr_session.clone(), true, true),
                ) {
                    self.face_sources.face_expressions_tracker =
                        Some(FaceExpressionsTracker::Fb(tracker))
                } else if let Some(tracker) = check_ext_object(
                    "FaceTrackerPico",
                    FaceTrackerPico::new(self.xr_session.clone()),
                ) {
                    tracker.start_face_tracking().ok();

                    self.face_sources.face_expressions_tracker =
                        Some(FaceExpressionsTracker::Pico(tracker));
                }
                // For vive, face trackers are always created at startup regardless of settings, and
                // also cannot be destroyed early.
            }
        }

        if let Some(config) = &config.body_tracking {
            if config.meta.prefer_full_body {
                self.body_source = check_ext_object(
                    "BodyTrackerFB (full set)",
                    BodyTrackerFB::new(
                        &self.xr_session,
                        self.xr_system,
                        *BODY_JOINT_SET_FULL_BODY_META,
                        config.meta.prefer_high_fidelity,
                    ),
                )
                .map(|tracker| BodyTracker::Fb {
                    tracker,
                    joint_count: FULL_BODY_JOINT_COUNT_META,
                });
            }
            if self.body_source.is_none() {
                self.body_source = check_ext_object(
                    "BodyTrackerFB (default set)",
                    BodyTrackerFB::new(
                        &self.xr_session,
                        self.xr_system,
                        xr::BodyJointSetFB::DEFAULT,
                        config.meta.prefer_high_fidelity,
                    ),
                )
                .map(|tracker| BodyTracker::Fb {
                    tracker,
                    joint_count: xr::BodyJointFB::COUNT.into_raw() as usize,
                });
            }
            if self.body_source.is_none() {
                match config.bd {
                    BodyTrackingBDConfig::BodyTracking {
                        high_accuracy,
                        prompt_calibration_on_start,
                    } => {
                        if high_accuracy {
                            self.body_source = check_ext_object(
                                "BodyTrackerBD (high accuracy)",
                                BodyTrackerBD::new(
                                    self.xr_session.clone(),
                                    BodyJointSetBD::FULL_BODY_JOINTS,
                                    &self.extra_extensions,
                                    self.xr_system,
                                    prompt_calibration_on_start,
                                ),
                            )
                            .map(BodyTracker::BodyBD);
                        }
                        if self.body_source.is_none() {
                            self.body_source = check_ext_object(
                                "BodyTrackerBD (low accuracy)",
                                BodyTrackerBD::new(
                                    self.xr_session.clone(),
                                    BodyJointSetBD::BODY_WITHOUT_ARM,
                                    &self.extra_extensions,
                                    self.xr_system,
                                    prompt_calibration_on_start,
                                ),
                            )
                            .map(BodyTracker::BodyBD);
                        }
                    }
                    BodyTrackingBDConfig::ObjectTracking => {
                        self.body_source = check_ext_object(
                            "MotionTrackerBD (object tracking)",
                            MotionTrackerBD::new(self.xr_session.clone(), &self.extra_extensions),
                        )
                        .map(BodyTracker::MotionBD);
                    }
                }
            }
        }
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

pub fn get_head_data(
    xr_session: &xr::Session<xr::OpenGlEs>,
    platform: Platform,
    stage_reference_space: &xr::Space,
    view_reference_space: &xr::Space,
    time: Duration,
    future_time: Duration,
    last_view_params: &[ViewParams; 2],
) -> Option<(DeviceMotion, Option<[ViewParams; 2]>)> {
    let xr_time = crate::to_xr_time(time);

    let (head_location, head_velocity) = view_reference_space
        .relate(stage_reference_space, xr_time)
        .ok()?;

    if !head_location
        .location_flags
        .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
    {
        return None;
    }

    let (view_flags, views) = xr_session
        .locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            xr_time,
            stage_reference_space,
        )
        .ok()?;

    if !view_flags.contains(xr::ViewStateFlags::POSITION_VALID)
        || !view_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
    {
        return None;
    }

    let mut motion = DeviceMotion {
        pose: crate::from_xr_pose(head_location.pose),
        linear_velocity: if head_velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
        {
            crate::from_xr_vec3(head_velocity.linear_velocity)
        } else {
            Vec3::ZERO
        },
        angular_velocity: if head_velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
        {
            crate::from_xr_vec3(head_velocity.angular_velocity)
        } else {
            Vec3::ZERO
        },
    };

    // Some headsets use wrong frame of reference for linear and angular velocities.
    if platform.is_pico() || platform.is_vive() || platform.is_yvr() {
        let xr_future_time = crate::to_xr_time(future_time);

        let predicted_location = view_reference_space
            .locate(stage_reference_space, xr_future_time)
            .ok()?;

        if !predicted_location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
        {
            return None;
        }

        let time_offset = future_time.saturating_sub(time);

        if !time_offset.is_zero() {
            let time_offset_s = time_offset.as_secs_f32();

            motion.linear_velocity = (crate::from_xr_vec3(predicted_location.pose.position)
                - motion.pose.position)
                / time_offset_s;
            motion.angular_velocity = (crate::from_xr_quat(predicted_location.pose.orientation)
                * motion.pose.orientation.inverse())
            .to_scaled_axis()
                / time_offset_s;
        }
    }

    let last_ipd_m = last_view_params[0]
        .pose
        .position
        .distance(last_view_params[1].pose.position);
    let current_ipd_m = crate::from_xr_vec3(views[1].pose.position)
        .distance(crate::from_xr_vec3(views[0].pose.position));
    let view_params = if f32::abs(current_ipd_m - last_ipd_m) > IPD_CHANGE_EPS {
        Some([
            ViewParams {
                pose: motion.pose.inverse() * crate::from_xr_pose(views[0].pose),
                fov: crate::from_xr_fov(views[0].fov),
            },
            ViewParams {
                pose: motion.pose.inverse() * crate::from_xr_pose(views[1].pose),
                fov: crate::from_xr_fov(views[1].fov),
            },
        ])
    } else {
        None
    };

    Some((motion, view_params))
}

#[expect(clippy::too_many_arguments)]
pub fn get_hand_data(
    xr_session: &xr::Session<xr::OpenGlEs>,
    platform: Platform,
    reference_space: &xr::Space,
    time: Duration,
    future_time: Duration,
    hand_source: &HandInteraction,
    last_controller_pose: &mut Pose,
    last_palm_pose: &mut Pose,
) -> HandData {
    let xr_time = crate::to_xr_time(time);
    let xr_now = crate::xr_runtime_now(xr_session.instance()).unwrap_or(xr_time);

    let grip_motion = if hand_source
        .grip_action
        .is_active(xr_session, xr::Path::NULL)
        .unwrap_or(false)
        && let Ok((location, velocity)) = hand_source.grip_space.relate(reference_space, xr_time)
    {
        let orientation_valid = location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID);
        let position_valid = location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_VALID);

        if orientation_valid {
            last_controller_pose.orientation = crate::from_xr_quat(location.pose.orientation);
        }

        if position_valid {
            last_controller_pose.position = crate::from_xr_vec3(location.pose.position);
        }

        let pose = *last_controller_pose * hand_source.pose_offset;

        let mut linear_velocity = crate::from_xr_vec3(velocity.linear_velocity);
        let mut angular_velocity = crate::from_xr_vec3(velocity.angular_velocity);

        let time_offset = future_time.saturating_sub(time);

        // Some headsets use wrong frame of reference for linear and angular velocities.
        if (platform.is_pico() || platform.is_vive())
            && !time_offset.is_zero()
            && let Ok(future_location) = hand_source
                .grip_space
                .locate(reference_space, crate::to_xr_time(future_time))
            && future_location.location_flags.contains(
                xr::SpaceLocationFlags::ORIENTATION_VALID | xr::SpaceLocationFlags::POSITION_VALID,
            )
        {
            let time_offset_s = time_offset.as_secs_f32();

            linear_velocity = (crate::from_xr_vec3(future_location.pose.position)
                - last_controller_pose.position)
                / time_offset_s;
            angular_velocity = (crate::from_xr_quat(future_location.pose.orientation)
                * last_controller_pose.orientation.inverse())
            .to_scaled_axis()
                / time_offset_s;
        }

        Some(DeviceMotion {
            pose,
            linear_velocity,
            angular_velocity,
        })
    } else {
        None
    };

    let detached_grip_motion = if let Some(detached_grip_action) = &hand_source.detached_grip_action
        && detached_grip_action
            .is_active(xr_session, xr::Path::NULL)
            .unwrap_or(false)
        && let Ok((location, velocity)) = hand_source
            .detached_grip_space
            .as_ref()
            .unwrap()
            .relate(reference_space, xr_time)
    {
        if location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
        {
            last_controller_pose.orientation = crate::from_xr_quat(location.pose.orientation);
        }

        if location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_VALID)
        {
            last_controller_pose.position = crate::from_xr_vec3(location.pose.position);
        }

        Some(DeviceMotion {
            pose: *last_controller_pose,
            linear_velocity: crate::from_xr_vec3(velocity.linear_velocity),
            angular_velocity: crate::from_xr_vec3(velocity.angular_velocity),
        })
    } else {
        None
    };

    let skeleton_joints = if let Some(tracker) = &hand_source.skeleton_tracker
        && let Some(joint_locations) = reference_space
            .locate_hand_joints(tracker, xr_now)
            .ok()
            .flatten()
    {
        if joint_locations[0]
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
        {
            last_palm_pose.orientation = crate::from_xr_quat(joint_locations[0].pose.orientation);
        }

        if joint_locations[0]
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_VALID)
        {
            last_palm_pose.position = crate::from_xr_vec3(joint_locations[0].pose.position);
        }

        let mut joints: [_; 26] = joint_locations
            .iter()
            .map(|j| crate::from_xr_pose(j.pose))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        joints[0] = *last_palm_pose;

        Some(joints)
    } else {
        None
    };

    HandData {
        grip_motion,
        detached_grip_motion,
        skeleton_joints,
    }
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

// Note: Using the headset view space in order to get heading-independent eye gazes
pub fn get_face_data(
    xr_session: &xr::Session<xr::OpenGlEs>,
    sources: &FaceSources,
    view_reference_space: &xr::Space,
    time: Duration,
) -> FaceData {
    let xr_time = crate::to_xr_time(time);

    let eyes_combined = if let Some((action, space)) = &sources.eyes_combined
        && action
            .is_active(xr_session, xr::Path::NULL)
            .unwrap_or(false)
        && let Ok(location) = space.locate(view_reference_space, xr_time)
        && location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
    {
        Some(crate::from_xr_quat(location.pose.orientation))
    } else {
        None
    };

    let eyes_social = if let Some(tracker) = &sources.eyes_social
        && let Ok(gazes) = tracker.get_eye_gazes(view_reference_space, xr_time)
    {
        [
            gazes[0].map(|p| crate::from_xr_quat(p.orientation)),
            gazes[1].map(|p| crate::from_xr_quat(p.orientation)),
        ]
    } else {
        [None, None]
    };

    let face_expressions = if let Some(tracker) = &sources.face_expressions_tracker {
        match tracker {
            FaceExpressionsTracker::Fb(tracker) => tracker
                .get_face_expression_weights(xr_time)
                .ok()
                .flatten()
                .map(|weights| FaceExpressions::Fb(weights.into_iter().collect())),
            FaceExpressionsTracker::Pico(face_tracker_pico) => face_tracker_pico
                .get_face_tracking_data(xr_time)
                .ok()
                .flatten()
                .map(|weights| FaceExpressions::Pico(weights.into_iter().collect())),
            FaceExpressionsTracker::Htc { eye, lip } => {
                let eye = eye
                    .as_ref()
                    .and_then(|tracker| tracker.get_facial_expressions(xr_time).ok().flatten());
                let lip = lip
                    .as_ref()
                    .and_then(|tracker| tracker.get_facial_expressions(xr_time).ok().flatten());

                Some(FaceExpressions::Htc { eye, lip })
            }
        }
    } else {
        None
    };

    FaceData {
        eyes_combined,
        eyes_social,
        face_expressions,
    }
}

pub fn get_body_skeleton(
    source: &BodyTracker,
    reference_space: &xr::Space,
    time: Duration,
) -> Option<BodySkeleton> {
    let xr_time = crate::to_xr_time(time);

    let check_and_convert_pose = |pose, location_flags: &xr::SpaceLocationFlags| {
        if location_flags
            .contains(SpaceLocationFlags::ORIENTATION_VALID | SpaceLocationFlags::POSITION_VALID)
        {
            Some(crate::from_xr_pose(pose))
        } else {
            None
        }
    };

    match source {
        BodyTracker::Fb {
            tracker,
            joint_count,
        } => {
            if let Some(joints) = tracker
                .locate_body_joints(xr_time, reference_space, *joint_count)
                .ok()
                .flatten()
            {
                let joints = joints
                    .iter()
                    .map(|joint| check_and_convert_pose(joint.pose, &joint.location_flags))
                    .collect::<Vec<_>>();

                Some(BodySkeleton::Fb(Box::new(BodySkeletonFb {
                    upper_body: joints[..18].try_into().unwrap(),
                    lower_body: (joints.len() >= 84).then(|| joints[70..84].try_into().unwrap()),
                })))
            } else {
                None
            }
        }
        BodyTracker::BodyBD(tracker) => {
            if let Some(joints) = tracker
                .locate_body_joints(xr_time, reference_space)
                .ok()
                .flatten()
            {
                let joints = joints
                    .iter()
                    .map(|joint| check_and_convert_pose(joint.pose, &joint.location_flags))
                    .collect::<Vec<_>>();

                Some(BodySkeleton::Bd(Box::new(BodySkeletonBd(
                    joints.try_into().unwrap(),
                ))))
            } else {
                None
            }
        }
        // Motion trackers are polled separately
        BodyTracker::MotionBD(_) => None,
    }
}

pub fn get_bd_motion_trackers(source: &BodyTracker, time: Duration) -> Vec<(u64, DeviceMotion)> {
    let xr_time = crate::to_xr_time(time);

    if let BodyTracker::MotionBD(tracker) = source
        && let Some(mut trackers) = tracker.locate_motion_trackers(xr_time).ok().flatten()
    {
        let mut joints = Vec::<(u64, DeviceMotion)>::with_capacity(3);

        let joints_ids = [
            *GENERIC_TRACKER_1_ID,
            *GENERIC_TRACKER_2_ID,
            *GENERIC_TRACKER_3_ID,
        ];

        trackers.sort_by(|a, b| a.serial.cmp(&b.serial));

        for (i, item) in trackers.iter().enumerate() {
            joints.push((
                joints_ids[i],
                DeviceMotion {
                    pose: crate::from_xr_pose(item.local_pose.pose),
                    linear_velocity: crate::from_xr_vec3(item.local_pose.linear_velocity),
                    angular_velocity: crate::from_xr_vec3(item.local_pose.angular_velocity),
                },
            ))
        }

        return joints;
    }

    Vec::new()
}
