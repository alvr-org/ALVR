#![allow(
    non_upper_case_globals,
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::unseparated_literal_suffix
)]

mod c_api;
mod connection;
mod decoder;
mod logging_backend;
mod platform;
mod sockets;
mod statistics;
mod storage;

pub mod opengl;

#[cfg(target_os = "android")]
mod audio;

use alvr_common::{
    error,
    glam::{UVec2, Vec2},
    parking_lot::{Mutex, RwLock},
    ConnectionState, Fov, LifecycleState,
};
use alvr_packets::{
    BatteryPacket, ButtonEntry, ClientControlPacket, NegotiatedStreamingConfig,
    ReservedClientControlPacket, Tracking, ViewsConfig,
};
use alvr_session::{CodecType, Settings};
use connection::ConnectionContext;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use storage::Config;

pub use platform::Platform;

#[derive(Serialize, Deserialize)]
pub enum ClientCoreEvent {
    UpdateHudMessage(String),
    StreamingStarted {
        settings: Box<Settings>,
        negotiated_config: NegotiatedStreamingConfig,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
    // Note: All subsequent DecoderConfig events should be ignored until reconnection
    DecoderConfig {
        codec: CodecType,
        config_nal: Vec<u8>,
    },
    FrameReady {
        timestamp: Duration,
        nal: Vec<u8>,
    },
}

pub fn platform() -> Platform {
    platform::platform()
}

// Note: this struct may change without breaking network protocol changes
#[derive(Clone)]
pub struct ClientCapabilities {
    pub default_view_resolution: UVec2,
    pub external_decoder: bool,
    pub refresh_rates: Vec<f32>,
    pub foveated_encoding: bool,
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
}

pub use logging_backend::init_logging;

#[cfg(target_os = "android")]
pub use platform::try_get_permission;

pub struct ClientCoreContext {
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    event_queue: Arc<Mutex<VecDeque<ClientCoreEvent>>>,
    connection_context: Arc<ConnectionContext>,
    connection_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ClientCoreContext {
    pub fn new(capabilities: ClientCapabilities) -> Self {
        // Make sure to reset config in case of version compat mismatch.
        if Config::load().protocol_id != alvr_common::protocol_id() {
            // NB: Config::default() sets the current protocol ID
            Config::default().store();
        }

        #[cfg(target_os = "android")]
        platform::try_get_permission(platform::MICROPHONE_PERMISSION);
        #[cfg(target_os = "android")]
        platform::set_wifi_lock(true);

        let lifecycle_state = Arc::new(RwLock::new(LifecycleState::Idle));
        let event_queue = Arc::new(Mutex::new(VecDeque::new()));
        let connection_context = Arc::new(ConnectionContext::default());
        let connection_thread = thread::spawn({
            let lifecycle_state = Arc::clone(&lifecycle_state);
            let connection_context = Arc::clone(&connection_context);
            let event_queue = Arc::clone(&event_queue);
            move || {
                connection::connection_lifecycle_loop(
                    capabilities,
                    connection_context,
                    lifecycle_state,
                    event_queue,
                )
            }
        });

        Self {
            lifecycle_state,
            event_queue,
            connection_context,
            connection_thread: Arc::new(Mutex::new(Some(connection_thread))),
        }
    }

    pub fn resume(&self) {
        *self.lifecycle_state.write() = LifecycleState::Resumed;
    }

    pub fn pause(&self) {
        let mut connection_state_lock = self.connection_context.state.write();

        *self.lifecycle_state.write() = LifecycleState::Idle;

        // We want to shutdown streaming when pausing.
        if *connection_state_lock != ConnectionState::Disconnected {
            alvr_common::wait_rwlock(
                &self.connection_context.disconnected_notif,
                &mut connection_state_lock,
            );
        }
    }

    pub fn poll_event(&self) -> Option<ClientCoreEvent> {
        self.event_queue.lock().pop_front()
    }

    pub fn send_views_config(&self, fov: [Fov; 2], ipd_m: f32) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::ViewsConfig(ViewsConfig {
                    fov,
                    ipd_m,
                }))
                .ok();
        }
    }

    pub fn send_battery(&self, device_id: u64, gauge_value: f32, is_plugged: bool) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::Battery(BatteryPacket {
                    device_id,
                    gauge_value,
                    is_plugged,
                }))
                .ok();
        }
    }

    pub fn send_playspace(&self, area: Option<Vec2>) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::PlayspaceSync(area)).ok();
        }
    }

    pub fn send_active_interaction_profile(&self, device_id: u64, profile_id: u64) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::ActiveInteractionProfile {
                    device_id,
                    profile_id,
                })
                .ok();
        }
    }

    pub fn send_custom_interaction_profile(&self, device_id: u64, input_ids: HashSet<u64>) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&alvr_packets::encode_reserved_client_control_packet(
                    &ReservedClientControlPacket::CustomInteractionProfile {
                        device_id,
                        input_ids,
                    },
                ))
                .ok();
        }
    }

    pub fn send_buttons(&self, entries: Vec<ButtonEntry>) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::Buttons(entries)).ok();
        }
    }

    pub fn send_tracking(&self, tracking: Tracking) {
        if let Some(sender) = &mut *self.connection_context.tracking_sender.lock() {
            sender.send_header(&tracking).ok();

            if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
                stats.report_input_acquired(tracking.target_timestamp);
            }
        }
    }

    pub fn get_head_prediction_offset(&self) -> Duration {
        if let Some(stats) = &*self.connection_context.statistics_manager.lock() {
            stats.average_total_pipeline_latency()
        } else {
            Duration::ZERO
        }
    }

    pub fn get_tracker_prediction_offset(&self) -> Duration {
        if let Some(stats) = &*self.connection_context.statistics_manager.lock() {
            stats.tracker_prediction_offset()
        } else {
            Duration::ZERO
        }
    }

    pub fn get_frame(&self) -> Option<(Duration, *mut std::ffi::c_void)> {
        if let Some(source) = &mut *self.connection_context.decoder_source.lock() {
            if let Some((timestamp, buffer_ptr)) = source.get_frame() {
                if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
                    stats.report_compositor_start(timestamp);
                }

                Some((timestamp, buffer_ptr))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Call only with external decoder
    pub fn request_idr(&self) {
        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::RequestIdr).ok();
        }
    }

    /// Call only with external decoder
    pub fn report_frame_decoded(&self, target_timestamp: Duration) {
        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_frame_decoded(target_timestamp);
        }
    }

    /// Call only with external decoder
    pub fn report_compositor_start(&self, target_timestamp: Duration) {
        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_compositor_start(target_timestamp);
        }
    }

    pub fn report_submit(&self, target_timestamp: Duration, vsync_queue: Duration) {
        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_submit(target_timestamp, vsync_queue);

            if let Some(sender) = &mut *self.connection_context.statistics_sender.lock() {
                if let Some(stats) = stats.summary(target_timestamp) {
                    sender.send_header(&stats).ok();
                } else {
                    error!("Statistics summary not ready!");
                }
            }
        }
    }
}

impl Drop for ClientCoreContext {
    fn drop(&mut self) {
        *self.lifecycle_state.write() = LifecycleState::ShuttingDown;

        if let Some(thread) = self.connection_thread.lock().take() {
            thread.join().ok();
        }

        #[cfg(target_os = "android")]
        platform::set_wifi_lock(false);
    }
}
