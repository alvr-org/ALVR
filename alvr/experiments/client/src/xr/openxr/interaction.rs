use super::{convert, SceneButtons, XrContext, XrViewConfig};
use crate::xr::{XrActionType, XrActionValue, XrProfileDesc};
use alvr_common::prelude::*;
use alvr_sockets::MotionData;
use openxr as xr;
use std::collections::HashMap;

const OCULUS_PROFILE: &str = "/interaction_profiles/oculus/touch_controller";

const SELECT_ACTION_NAME: &str = "alvr_scene_select";
const OCULUS_SELECT_PATHS: &[&str] = &[
    "/user/hand/left/input/x/click",
    "/user/hand/right/input/a/click",
    "/user/hand/left/input/trigger",
    "/user/hand/right/input/trigger",
];

const MENU_ACTION_NAME: &str = "alvr_scene_menu";
const OCULUS_MENU_PATHS: &[&str] = &["/user/hand/left/input/menu/click"];

enum OpenxrButtonAction {
    Binary(xr::Action<bool>),
    Scalar(xr::Action<f32>),
}

pub struct TrackerContext {
    // Only one pose action per tracker. For controllers, grip action is used
    pose_action: xr::Action<xr::Posef>,
    space: xr::Space,
    vibration_action: xr::Action<xr::Haptic>,
}

pub struct XrInteractionContext {
    session: xr::Session<xr::Vulkan>,
    action_set: xr::ActionSet,
    scene_select_action: xr::Action<bool>,
    scene_menu_action: xr::Action<bool>,
    streaming_button_actions: HashMap<u64, OpenxrButtonAction>,
    pub reference_space: xr::Space,
    pub left_hand_tracker_context: TrackerContext,
    pub right_hand_tracker_context: TrackerContext,
    pub left_hand_skeleton_tracker: Option<xr::HandTracker>,
    pub right_hand_skeleton_tracker: Option<xr::HandTracker>,
    // todo: vive trackers
}

impl XrInteractionContext {
    fn get_hand_interaction(
        xr_context: &XrContext,
        session: xr::Session<xr::Vulkan>,
        action_set: &xr::ActionSet,
        hand: xr::Hand,
    ) -> StrResult<(TrackerContext, Option<xr::HandTracker>)> {
        let hand_str = if hand == xr::Hand::LEFT {
            "alvr_left"
        } else {
            "alvr_right"
        };

        let pose_action_name = format!("{hand_str}_grip");
        let pose_action =
            trace_err!(action_set.create_action(&pose_action_name, &pose_action_name, &[]))?;
        let space = trace_err!(pose_action.create_space(
            session.clone(),
            xr::Path::NULL,
            xr::Posef::IDENTITY
        ))?;

        let skeleton_tracking_context = if trace_err!(xr_context
            .instance
            .supports_hand_tracking(xr_context.system))?
        {
            Some(trace_err!(session.create_hand_tracker(hand))?)
        } else {
            None
        };

        let vibration_action_name = format!("{hand_str}_haptics");
        let vibration_action = trace_err!(action_set.create_action(
            &vibration_action_name,
            &vibration_action_name,
            &[]
        ))?;

        Ok((
            TrackerContext {
                pose_action,
                space,
                vibration_action,
            },
            skeleton_tracking_context,
        ))
    }

    pub fn new(
        xr_context: &XrContext,
        session: xr::Session<xr::Vulkan>,
        stream_action_types: &[(String, XrActionType)],
        stream_profile_descs: Vec<XrProfileDesc>,
    ) -> StrResult<Self> {
        let action_set =
            trace_err!(xr_context
                .instance
                .create_action_set("alvr_bindings", "ALVR bindings", 0))?;

        let mut button_actions = HashMap::new();
        button_actions.insert(
            SELECT_ACTION_NAME.to_owned(),
            OpenxrButtonAction::Binary(trace_err!(action_set.create_action(
                SELECT_ACTION_NAME,
                SELECT_ACTION_NAME,
                &[]
            ))?),
        );
        button_actions.insert(
            MENU_ACTION_NAME.to_owned(),
            OpenxrButtonAction::Binary(trace_err!(action_set.create_action(
                MENU_ACTION_NAME,
                MENU_ACTION_NAME,
                &[]
            ))?),
        );

        for (name, action_type) in stream_action_types {
            match action_type {
                XrActionType::Binary => button_actions.insert(
                    name.clone(),
                    OpenxrButtonAction::Binary(trace_err!(action_set.create_action(
                        name,
                        name,
                        &[]
                    ))?),
                ),
                XrActionType::Scalar => button_actions.insert(
                    name.clone(),
                    OpenxrButtonAction::Scalar(trace_err!(action_set.create_action(
                        name,
                        name,
                        &[]
                    ))?),
                ),
            };
        }

        let (left_hand_tracker_context, left_hand_tracking_context) =
            Self::get_hand_interaction(xr_context, session.clone(), &action_set, xr::Hand::LEFT)?;
        let (right_hand_tracker_context, right_hand_tracking_context) =
            Self::get_hand_interaction(xr_context, session.clone(), &action_set, xr::Hand::RIGHT)?;

        let mut profile_descs = vec![];
        for mut profile in stream_profile_descs {
            if profile.profile == OCULUS_PROFILE {
                profile.tracked = true;
                profile.has_haptics = true;

                for path in OCULUS_SELECT_PATHS {
                    profile
                        .button_bindings
                        .push((SELECT_ACTION_NAME.to_owned(), (*path).to_owned()));
                }
                for path in OCULUS_MENU_PATHS {
                    profile
                        .button_bindings
                        .push((MENU_ACTION_NAME.to_owned(), (*path).to_owned()));
                }
            }

            profile_descs.push(profile);
        }
        if profile_descs
            .iter()
            .any(|profile| profile.profile == OCULUS_PROFILE)
        {
            let mut button_bindings = vec![];

            for path in OCULUS_SELECT_PATHS {
                button_bindings.push((SELECT_ACTION_NAME.to_owned(), (*path).to_owned()));
            }
            for path in OCULUS_MENU_PATHS {
                button_bindings.push((MENU_ACTION_NAME.to_owned(), (*path).to_owned()));
            }

            profile_descs.push(XrProfileDesc {
                profile: OCULUS_PROFILE.to_owned(),
                button_bindings,
                tracked: true,
                has_haptics: true,
            })
        }

        for profile in profile_descs {
            let profile_path = trace_err!(xr_context.instance.string_to_path(&profile.profile))?;

            let mut bindings = vec![];

            for (action_name, path_string) in &profile.button_bindings {
                let action = if let Some(res) = button_actions.get(action_name) {
                    res
                } else {
                    return fmt_e!("Action {action_name} not defined");
                };
                let path = trace_err!(xr_context.instance.string_to_path(path_string))?;

                match action {
                    OpenxrButtonAction::Binary(action) => {
                        bindings.push(xr::Binding::new(action, path))
                    }
                    OpenxrButtonAction::Scalar(action) => {
                        bindings.push(xr::Binding::new(action, path))
                    }
                }
            }

            if profile.tracked {
                bindings.push(xr::Binding::new(
                    &left_hand_tracker_context.pose_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/left/input/grip/pose"))?,
                ));

                bindings.push(xr::Binding::new(
                    &right_hand_tracker_context.pose_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/right/input/grip/pose"))?,
                ));
            }

            if profile.has_haptics {
                bindings.push(xr::Binding::new(
                    &left_hand_tracker_context.vibration_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/left/output/haptic"))?,
                ));
                bindings.push(xr::Binding::new(
                    &right_hand_tracker_context.vibration_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/right/output/haptic"))?,
                ));
            }

            // Ignore error for unsupported profiles
            xr_context
                .instance
                .suggest_interaction_profile_bindings(profile_path, &bindings)
                .ok();
        }

        trace_err!(session.attach_action_sets(&[&action_set]))?;

        let reference_space = trace_err!(session
            .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
            .or_else(|_| {
                session.create_reference_space(
                    xr::ReferenceSpaceType::LOCAL,
                    xr::Posef {
                        orientation: xr::Quaternionf::IDENTITY,
                        position: xr::Vector3f {
                            x: 0.0,
                            y: -1.5, // todo: make configurable
                            z: 0.0,
                        },
                    },
                )
            }))?;

        let scene_select_action = match button_actions.remove(SELECT_ACTION_NAME).unwrap() {
            OpenxrButtonAction::Binary(action) => action,
            _ => unreachable!(),
        };
        let scene_menu_action = match button_actions.remove(MENU_ACTION_NAME).unwrap() {
            OpenxrButtonAction::Binary(action) => action,
            _ => unreachable!(),
        };

        let streaming_button_actions = button_actions
            .into_iter()
            .map(|(name, action)| (alvr_common::hash_string(&name), action))
            .collect();

        Ok(Self {
            session,
            action_set,
            scene_select_action,
            scene_menu_action,
            streaming_button_actions,
            reference_space,
            left_hand_tracker_context,
            right_hand_tracker_context,
            left_hand_skeleton_tracker: left_hand_tracking_context,
            right_hand_skeleton_tracker: right_hand_tracking_context,
        })
    }

    pub fn sync_input(&self) -> StrResult {
        trace_err!(self.session.sync_actions(&[(&self.action_set).into()]))
    }

    pub fn get_views(
        &self,
        view_configuration_type: xr::ViewConfigurationType,
        display_time: xr::Time,
    ) -> StrResult<Vec<XrViewConfig>> {
        let (_, views) = trace_err!(self.session.locate_views(
            view_configuration_type,
            display_time,
            &self.reference_space
        ))?;

        Ok(views
            .into_iter()
            .map(|view| XrViewConfig {
                orientation: convert::from_xr_orientation(view.pose.orientation),
                position: convert::from_xr_vec3(view.pose.position),
                fov: convert::from_xr_fov(view.fov),
            })
            .collect())
    }

    fn get_motion(location: xr::SpaceLocation, velocity: xr::SpaceVelocity) -> MotionData {
        MotionData {
            orientation: convert::from_xr_orientation(location.pose.orientation),
            position: convert::from_xr_vec3(location.pose.position),
            linear_velocity: convert::from_xr_vec3(velocity.linear_velocity),
            angular_velocity: convert::from_xr_vec3(velocity.angular_velocity),
        }
    }

    pub fn get_tracker_pose(
        &self,
        context: &TrackerContext,
        display_time: xr::Time,
    ) -> StrResult<MotionData> {
        let (location, velocity) =
            trace_err!(context.space.relate(&self.reference_space, display_time))?;
        Ok(Self::get_motion(location, velocity))
    }

    pub fn get_hand_skeleton(
        &self,
        hand_tracker: &xr::HandTracker,
        display_time: xr::Time,
    ) -> StrResult<Option<[MotionData; 26]>> {
        if let Some((joint_locations, joint_velocities)) = trace_err!(self
            .reference_space
            .relate_hand_joints(hand_tracker, display_time))?
        {
            let skeleton_motion = joint_locations
                .into_iter()
                .zip(joint_velocities.into_iter())
                .map(|(joint_location, joint_velocity)| MotionData {
                    orientation: convert::from_xr_orientation(joint_location.pose.orientation),
                    position: convert::from_xr_vec3(joint_location.pose.position),
                    linear_velocity: convert::from_xr_vec3(joint_velocity.linear_velocity),
                    angular_velocity: convert::from_xr_vec3(joint_velocity.angular_velocity),
                })
                .collect::<Vec<_>>();

            Ok(Some(skeleton_motion.try_into().unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn get_scene_buttons(&self) -> StrResult<SceneButtons> {
        let select_state = trace_err!(self
            .scene_select_action
            .state(&self.session, xr::Path::NULL))?;
        let menu_state = trace_err!(self.scene_menu_action.state(&self.session, xr::Path::NULL))?;

        Ok(SceneButtons {
            select: select_state.current_state,
            menu: menu_state.current_state,
        })
    }

    pub fn get_streming_buttons(&self) -> StrResult<Vec<(u64, XrActionValue)>> {
        let mut values = Vec::new();

        for (hash, action) in &self.streaming_button_actions {
            match action {
                OpenxrButtonAction::Binary(action) => {
                    values.push((
                        *hash,
                        XrActionValue::Boolean(
                            trace_err!(action.state(&self.session, xr::Path::NULL))?.current_state,
                        ),
                    ));
                }
                OpenxrButtonAction::Scalar(action) => {
                    values.push((
                        *hash,
                        XrActionValue::Scalar(
                            trace_err!(action.state(&self.session, xr::Path::NULL))?.current_state,
                        ),
                    ));
                }
            }
        }

        Ok(values)
    }
}
