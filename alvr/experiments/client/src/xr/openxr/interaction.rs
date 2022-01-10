use super::{convert, SceneButtons, XrContext, XrHandPoseInput};
use crate::{
    xr::{XrActionType, XrActionValue, XrHandTrackingInput, XrProfileDesc},
    ViewConfig,
};
use alvr_common::{prelude::*, MotionData};
use alvr_session::TrackingSpace;
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

struct HandTrackingContext {
    tracker: xr::HandTracker,

    // Note: target rays are used to have better aim when using HUD menus with hand tracking.
    // The target ray intersects the shoulder.
    target_ray_action: xr::Action<xr::Posef>,
    target_ray_space: xr::Space,
}

pub struct HandInteractionContext {
    grip_action: xr::Action<xr::Posef>,
    grip_space: xr::Space,
    hand_tracking_context: Option<HandTrackingContext>,
    vibration_action: xr::Action<xr::Haptic>,
}

pub struct OpenxrInteractionContext {
    session: xr::Session<xr::Vulkan>,
    action_set: xr::ActionSet,
    scene_select_action: xr::Action<bool>,
    scene_menu_action: xr::Action<bool>,
    streaming_button_actions: HashMap<String, OpenxrButtonAction>,
    pub reference_space: xr::Space,
    pub left_hand_interaction: HandInteractionContext,
    pub right_hand_interaction: HandInteractionContext,
}

impl OpenxrInteractionContext {
    fn get_hand_interaction(
        xr_context: &XrContext,
        session: xr::Session<xr::Vulkan>,
        action_set: &xr::ActionSet,
        hand: xr::Hand,
    ) -> StrResult<HandInteractionContext> {
        let hand_str = if hand == xr::Hand::LEFT {
            "alvr_left"
        } else {
            "alvr_right"
        };

        let grip_action_name = format!("{}_grip", hand_str);
        let grip_action =
            trace_err!(action_set.create_action(&grip_action_name, &grip_action_name, &[]))?;
        let grip_space = trace_err!(grip_action.create_space(
            session.clone(),
            xr::Path::NULL,
            xr::Posef::IDENTITY
        ))?;

        let hand_tracking_context = if trace_err!(xr_context
            .instance
            .supports_hand_tracking(xr_context.system))?
        {
            let tracker = trace_err!(session.create_hand_tracker(hand))?;

            let target_ray_action_name = format!("{}_aim", hand_str);
            let target_ray_action = trace_err!(action_set.create_action(
                &target_ray_action_name,
                &target_ray_action_name,
                &[]
            ))?;
            let target_ray_space = trace_err!(target_ray_action.create_space(
                session,
                xr::Path::NULL,
                xr::Posef::IDENTITY
            ))?;

            Some(HandTrackingContext {
                tracker,
                target_ray_action,
                target_ray_space,
            })
        } else {
            None
        };

        let vibration_action_name = format!("{}_haptics", hand_str);
        let vibration_action = trace_err!(action_set.create_action(
            &vibration_action_name,
            &vibration_action_name,
            &[]
        ))?;

        Ok(HandInteractionContext {
            grip_action,
            grip_space,
            hand_tracking_context,
            vibration_action,
        })
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

        let left_hand_interaction =
            Self::get_hand_interaction(xr_context, session.clone(), &action_set, xr::Hand::LEFT)?;
        let right_hand_interaction =
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
                    return fmt_e!("Action {} not defined", action_name);
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
                    &left_hand_interaction.grip_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/left/input/grip/pose"))?,
                ));
                if let Some(hand_tracking_context) = &left_hand_interaction.hand_tracking_context {
                    bindings.push(xr::Binding::new(
                        &hand_tracking_context.target_ray_action,
                        trace_err!(xr_context
                            .instance
                            .string_to_path("/user/hand/left/input/aim/pose"))?,
                    ));
                }

                bindings.push(xr::Binding::new(
                    &right_hand_interaction.grip_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/right/input/grip/pose"))?,
                ));
                if let Some(hand_tracking_context) = &right_hand_interaction.hand_tracking_context {
                    bindings.push(xr::Binding::new(
                        &hand_tracking_context.target_ray_action,
                        trace_err!(xr_context
                            .instance
                            .string_to_path("/user/hand/right/input/aim/pose"))?,
                    ));
                }
            }

            if profile.has_haptics {
                bindings.push(xr::Binding::new(
                    &left_hand_interaction.vibration_action,
                    trace_err!(xr_context
                        .instance
                        .string_to_path("/user/hand/left/output/haptic"))?,
                ));
                bindings.push(xr::Binding::new(
                    &right_hand_interaction.grip_action,
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

        Ok(Self {
            session,
            action_set,
            scene_select_action,
            scene_menu_action,
            streaming_button_actions: button_actions,
            reference_space,
            left_hand_interaction,
            right_hand_interaction,
        })
    }

    pub fn sync_input(&self) -> StrResult {
        trace_err!(self.session.sync_actions(&[(&self.action_set).into()]))
    }

    pub fn get_views(
        &self,
        view_configuration_type: xr::ViewConfigurationType,
        display_time: xr::Time,
    ) -> StrResult<Vec<ViewConfig>> {
        let (_, views) = trace_err!(self.session.locate_views(
            view_configuration_type,
            display_time,
            &self.reference_space
        ))?;

        Ok(views
            .into_iter()
            .map(|view| ViewConfig {
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
            linear_velocity: velocity.linear_velocity.map(convert::from_xr_vec3),
            angular_velocity: velocity.angular_velocity.map(convert::from_xr_vec3),
        }
    }

    pub fn get_poses(
        &self,
        hand_interaction: &HandInteractionContext,
        display_time: xr::Time,
    ) -> StrResult<XrHandPoseInput> {
        let (grip_location, grip_velocity) = trace_err!(hand_interaction
            .grip_space
            .relate(&self.reference_space, display_time))?;
        let grip_motion = Self::get_motion(grip_location, grip_velocity);

        let hand_tracking_input = if let Some(ctx) = &hand_interaction.hand_tracking_context {
            let (target_ray_location, target_ray_velocity) = trace_err!(ctx
                .target_ray_space
                .relate(&self.reference_space, display_time))?;
            let target_ray_motion = Self::get_motion(target_ray_location, target_ray_velocity);

            if let Some((joint_locations, joint_velocities)) = trace_err!(self
                .reference_space
                .relate_hand_joints(&ctx.tracker, display_time))?
            {
                let skeleton_motion = joint_locations
                    .iter()
                    .zip(joint_velocities.iter())
                    .map(|(joint_location, joint_velocity)| MotionData {
                        orientation: convert::from_xr_orientation(joint_location.pose.orientation),
                        position: convert::from_xr_vec3(joint_location.pose.position),
                        linear_velocity: joint_velocity
                            .velocity_flags
                            .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
                            .then(|| convert::from_xr_vec3(joint_velocity.linear_velocity)),
                        angular_velocity: joint_velocity
                            .velocity_flags
                            .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
                            .then(|| convert::from_xr_vec3(joint_velocity.angular_velocity)),
                    })
                    .collect();

                Some(XrHandTrackingInput {
                    target_ray_motion,
                    skeleton_motion,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(XrHandPoseInput {
            grip_motion,
            hand_tracking_input,
        })
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

    pub fn get_streming_buttons(&self) -> StrResult<HashMap<String, XrActionValue>> {
        let mut values = HashMap::new();

        for (name, action) in &self.streaming_button_actions {
            match action {
                OpenxrButtonAction::Binary(action) => {
                    values.insert(
                        name.clone(),
                        XrActionValue::Boolean(
                            trace_err!(action.state(&self.session, xr::Path::NULL))?.current_state,
                        ),
                    );
                }
                OpenxrButtonAction::Scalar(action) => {
                    values.insert(
                        name.clone(),
                        XrActionValue::Scalar(
                            trace_err!(action.state(&self.session, xr::Path::NULL))?.current_state,
                        ),
                    );
                }
            }
        }

        Ok(values)
    }
}
