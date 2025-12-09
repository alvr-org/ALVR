use openxr::{
    self as xr, AsHandle, raw,
    sys::{self, Handle},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, c_char},
    ptr,
    time::{Duration, Instant},
};

const SPATIAL_CAPABILITY: sys::SpatialCapabilityEXT =
    sys::SpatialCapabilityEXT::MARKER_TRACKING_QR_CODE;
// Note: The Meta implementation is currently bugged and the buffer capacity cannot be changed once
// the fist call of query_spatial_component_data is made.
const MAX_MARKERS_COUNT: usize = 32;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(1);

struct SpatialContextReadyData {
    handle: sys::SpatialContextEXT,
    discovery_snapshot_future: Option<sys::FutureEXT>,
    spatial_entities: HashMap<sys::SpatialEntityIdEXT, (String, sys::SpatialEntityEXT)>,
    entity_ids: [sys::SpatialEntityIdEXT; MAX_MARKERS_COUNT],
    entity_states: [sys::SpatialEntityTrackingStateEXT; MAX_MARKERS_COUNT],
    bounded_2d_arr: [sys::SpatialBounded2DDataEXT; MAX_MARKERS_COUNT],
    marker_arr: [sys::SpatialMarkerDataEXT; MAX_MARKERS_COUNT],
    string_buffer: [c_char; 256],
}

enum SpatialContextState {
    Creating(sys::FutureEXT),
    Ready(Box<SpatialContextReadyData>),
}

pub struct QRCodesSpatialContext {
    session: xr::Session<xr::OpenGlEs>,
    spatial_entity_fns: raw::SpatialEntityEXT,
    enabled_components: Vec<sys::SpatialComponentTypeEXT>,
    codes_to_track: HashSet<String>,
    context_state: SpatialContextState,
    discovery_timeout_deadline: Instant,
}

impl QRCodesSpatialContext {
    // If ids_to_track is empty, track all markers. This is used for the lobby
    pub fn new(
        session: &xr::Session<xr::OpenGlEs>,
        strings_to_track: HashSet<String>,
    ) -> xr::Result<Self> {
        let spatial_entity_fns = session
            .instance()
            .exts()
            .ext_spatial_entity
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;
        if session
            .instance()
            .exts()
            .ext_spatial_marker_tracking
            .is_none()
        {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let enabled_components = vec![
            sys::SpatialComponentTypeEXT::BOUNDED_2D,
            sys::SpatialComponentTypeEXT::MARKER,
        ];

        let qr_code_capability_configuration = sys::SpatialCapabilityConfigurationQrCodeEXT {
            ty: sys::SpatialCapabilityConfigurationQrCodeEXT::TYPE,
            next: ptr::null(),
            capability: SPATIAL_CAPABILITY,
            enabled_component_count: enabled_components.len() as u32,
            enabled_components: enabled_components.as_ptr(),
        };

        let base_capability_configuration = ptr::from_ref(&qr_code_capability_configuration).cast();
        let spatial_context_create_info = sys::SpatialContextCreateInfoEXT {
            ty: sys::SpatialContextCreateInfoEXT::TYPE,
            next: ptr::null(),
            capability_config_count: 1,
            capability_configs: &raw const base_capability_configuration,
        };

        let mut create_context_future: sys::FutureEXT = 0;
        unsafe {
            super::xr_res((spatial_entity_fns.create_spatial_context_async)(
                session.as_handle(),
                &spatial_context_create_info,
                &mut create_context_future,
            ))?;
        }

        Ok(QRCodesSpatialContext {
            session: session.clone(),
            spatial_entity_fns,
            enabled_components,
            codes_to_track: strings_to_track,
            context_state: SpatialContextState::Creating(create_context_future),
            discovery_timeout_deadline: Instant::now(),
        })
    }

    pub fn poll(
        &mut self,
        base_space: &xr::Space,
        time: xr::Time,
    ) -> xr::Result<Option<Vec<(String, xr::Posef)>>> {
        let now = Instant::now();

        let spatial_context_data = match &mut self.context_state {
            SpatialContextState::Ready(data) => data,

            SpatialContextState::Creating(future) => {
                if !super::check_future(self.session.instance(), *future)? {
                    return Ok(None);
                }

                let completion = unsafe {
                    let mut completion =
                        sys::CreateSpatialContextCompletionEXT::out(ptr::null_mut());
                    super::xr_res((self.spatial_entity_fns.create_spatial_context_complete)(
                        self.session.as_handle(),
                        *future,
                        completion.as_mut_ptr(),
                    ))?;
                    completion.assume_init()
                };
                if completion.future_result != sys::Result::SUCCESS {
                    return Err(completion.future_result);
                }

                self.context_state =
                    SpatialContextState::Ready(Box::new(SpatialContextReadyData {
                        handle: completion.spatial_context,
                        discovery_snapshot_future: None,
                        spatial_entities: HashMap::new(),
                        entity_ids: [sys::SpatialEntityIdEXT::NULL; MAX_MARKERS_COUNT],
                        entity_states: [sys::SpatialEntityTrackingStateEXT::STOPPED;
                            MAX_MARKERS_COUNT],
                        bounded_2d_arr: [sys::SpatialBounded2DDataEXT {
                            center: xr::Posef::IDENTITY,
                            extents: xr::Extent2Df::default(),
                        }; MAX_MARKERS_COUNT],
                        marker_arr: [sys::SpatialMarkerDataEXT {
                            capability: SPATIAL_CAPABILITY,
                            marker_id: 0,
                            data: sys::SpatialBufferEXT {
                                buffer_id: sys::SpatialBufferIdEXT::NULL,
                                buffer_type: sys::SpatialBufferTypeEXT::STRING,
                            },
                        }; MAX_MARKERS_COUNT],
                        string_buffer: [0; 256],
                    }));

                // Custom enums don't have the method insert(), so we cannot get back a mutable
                // reference directly.
                return Ok(None);
            }
        };

        // Try getting a discovery snapshot or submit query for one
        let snapshot_handle = if let &Some(future) = &spatial_context_data.discovery_snapshot_future
        {
            if super::check_future(self.session.instance(), future)? {
                spatial_context_data.discovery_snapshot_future = None;

                let completion_info = sys::CreateSpatialDiscoverySnapshotCompletionInfoEXT {
                    ty: sys::CreateSpatialDiscoverySnapshotCompletionInfoEXT::TYPE,
                    next: ptr::null(),
                    base_space: base_space.as_handle(),
                    time,
                    future,
                };

                let completion = unsafe {
                    let mut completion =
                        sys::CreateSpatialDiscoverySnapshotCompletionEXT::out(ptr::null_mut());
                    super::xr_res((self
                        .spatial_entity_fns
                        .create_spatial_discovery_snapshot_complete)(
                        spatial_context_data.handle,
                        &completion_info,
                        completion.as_mut_ptr(),
                    ))?;
                    completion.assume_init()
                };
                if completion.future_result != sys::Result::SUCCESS {
                    return Err(completion.future_result);
                }

                Some(completion.snapshot)
            } else {
                None
            }
        } else {
            if now > self.discovery_timeout_deadline {
                let snapshot_create_info = sys::SpatialDiscoverySnapshotCreateInfoEXT {
                    ty: sys::SpatialDiscoverySnapshotCreateInfoEXT::TYPE,
                    next: ptr::null(),
                    component_type_count: self.enabled_components.len() as u32,
                    component_types: self.enabled_components.as_ptr(),
                };

                let mut create_snapshot_future: sys::FutureEXT = 0;
                unsafe {
                    super::xr_res((self
                        .spatial_entity_fns
                        .create_spatial_discovery_snapshot_async)(
                        spatial_context_data.handle,
                        &snapshot_create_info,
                        &mut create_snapshot_future,
                    ))?;
                }

                spatial_context_data.discovery_snapshot_future = Some(create_snapshot_future);

                self.discovery_timeout_deadline = now + DISCOVERY_TIMEOUT;
            }

            None
        };

        // Get snapshot for already discovered entities if no discovery happened
        // Note: We are not generating a snapshot for the already discovered entities if we got a
        // discovery snapshot. This is because the
        let snapshot_handle = if let Some(handle) = snapshot_handle {
            handle
        } else {
            let entities = spatial_context_data
                .spatial_entities
                .values()
                .map(|(_, entity)| *entity)
                .collect::<Vec<sys::SpatialEntityEXT>>();
            let create_info = sys::SpatialUpdateSnapshotCreateInfoEXT {
                ty: sys::SpatialUpdateSnapshotCreateInfoEXT::TYPE,
                next: ptr::null(),
                entity_count: entities.len() as u32,
                entities: entities.as_ptr(),
                component_type_count: 0,
                component_types: ptr::null(),
                base_space: base_space.as_handle(),
                time,
            };

            let mut snapshot_handle = sys::SpatialSnapshotEXT::NULL;
            unsafe {
                (self.spatial_entity_fns.create_spatial_update_snapshot)(
                    spatial_context_data.handle,
                    &create_info,
                    &mut snapshot_handle,
                );
            }

            snapshot_handle
        };

        let query_contition = sys::SpatialComponentDataQueryConditionEXT {
            ty: sys::SpatialComponentDataQueryConditionEXT::TYPE,
            next: ptr::null(),
            component_type_count: self.enabled_components.len() as u32,
            component_types: self.enabled_components.as_ptr(),
        };

        let mut query_result = sys::SpatialComponentDataQueryResultEXT {
            ty: sys::SpatialComponentDataQueryResultEXT::TYPE,
            next: ptr::null_mut(),
            entity_id_capacity_input: MAX_MARKERS_COUNT as u32,
            entity_id_count_output: 0,
            entity_ids: spatial_context_data.entity_ids.as_mut_ptr(),
            entity_state_capacity_input: MAX_MARKERS_COUNT as u32,
            entity_state_count_output: 0,
            entity_states: spatial_context_data.entity_states.as_mut_ptr(),
        };
        unsafe {
            super::xr_res((self.spatial_entity_fns.query_spatial_component_data)(
                snapshot_handle,
                &query_contition,
                &mut query_result,
            ))?;
        }
        let marker_count = query_result.entity_id_count_output;

        let mut bounded_2d_list = sys::SpatialComponentBounded2DListEXT {
            ty: sys::SpatialComponentBounded2DListEXT::TYPE,
            next: ptr::null_mut(),
            bound_count: marker_count,
            bounds: spatial_context_data.bounded_2d_arr.as_mut_ptr(),
        };
        query_result.next = (&raw mut bounded_2d_list).cast();

        let mut marker_list = sys::SpatialComponentMarkerListEXT {
            ty: sys::SpatialComponentMarkerListEXT::TYPE,
            next: ptr::null_mut(),
            marker_count,
            markers: spatial_context_data.marker_arr.as_mut_ptr(),
        };
        bounded_2d_list.next = (&raw mut marker_list).cast();

        unsafe {
            super::xr_res((self.spatial_entity_fns.query_spatial_component_data)(
                snapshot_handle,
                &query_contition,
                &mut query_result,
            ))?;
        }

        let mut out_markers = vec![];
        for idx in 0..query_result.entity_id_count_output as usize {
            if spatial_context_data.entity_states[idx]
                != sys::SpatialEntityTrackingStateEXT::TRACKING
                || spatial_context_data.marker_arr[idx].capability != SPATIAL_CAPABILITY
                || spatial_context_data.marker_arr[idx].data.buffer_id
                    == sys::SpatialBufferIdEXT::NULL
                || spatial_context_data.marker_arr[idx].data.buffer_type
                    != sys::SpatialBufferTypeEXT::STRING
            {
                alvr_common::debug!(
                    "Parsing marker failed! {:?} {:?} {:?} {:?}",
                    spatial_context_data.entity_states[idx],
                    spatial_context_data.marker_arr[idx].capability,
                    spatial_context_data.marker_arr[idx].data.buffer_id,
                    spatial_context_data.marker_arr[idx].data.buffer_type
                );
                continue;
            }

            let entity_id = spatial_context_data.entity_ids[idx];

            let string = if let Some((string, _)) = spatial_context_data
                .spatial_entities
                .get(&spatial_context_data.entity_ids[idx])
            {
                string.clone()
            } else {
                let get_info = sys::SpatialBufferGetInfoEXT {
                    ty: sys::SpatialBufferGetInfoEXT::TYPE,
                    next: ptr::null(),
                    buffer_id: spatial_context_data.marker_arr[idx].data.buffer_id,
                };

                unsafe {
                    let mut _string_lenght = 0;
                    super::xr_res((self.spatial_entity_fns.get_spatial_buffer_string)(
                        snapshot_handle,
                        &get_info,
                        spatial_context_data.string_buffer.len() as u32,
                        &mut _string_lenght,
                        spatial_context_data.string_buffer.as_mut_ptr(),
                    ))?;
                };

                let string = unsafe { CStr::from_ptr(spatial_context_data.string_buffer.as_ptr()) }
                    .to_str()
                    .map_err(|_| sys::Result::ERROR_SPATIAL_BUFFER_ID_INVALID_EXT)?
                    .to_owned();

                if self.codes_to_track.is_empty() || self.codes_to_track.contains(&string) {
                    let create_info = sys::SpatialEntityFromIdCreateInfoEXT {
                        ty: sys::SpatialEntityFromIdCreateInfoEXT::TYPE,
                        next: ptr::null(),
                        entity_id,
                    };

                    let mut spatial_entity = sys::SpatialEntityEXT::NULL;
                    unsafe {
                        super::xr_res((self.spatial_entity_fns.create_spatial_entity_from_id)(
                            spatial_context_data.handle,
                            &create_info,
                            &mut spatial_entity,
                        ))?;
                    };

                    spatial_context_data
                        .spatial_entities
                        .insert(entity_id, (string.clone(), spatial_entity));
                }

                string
            };

            let pose = spatial_context_data.bounded_2d_arr[idx].center;
            out_markers.push((string, pose));
        }

        unsafe {
            super::xr_res((self.spatial_entity_fns.destroy_spatial_snapshot)(
                snapshot_handle,
            ))?;
        }

        Ok(Some(out_markers))
    }
}

impl Drop for QRCodesSpatialContext {
    fn drop(&mut self) {
        if let SpatialContextState::Ready(data) = &self.context_state {
            unsafe {
                super::xr_res((self.spatial_entity_fns.destroy_spatial_context)(
                    data.handle,
                ))
                .ok();
            }
        }
    }
}
