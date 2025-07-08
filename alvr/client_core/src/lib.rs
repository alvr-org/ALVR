#![allow(
    non_upper_case_globals,
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::unseparated_literal_suffix
)]

mod c_api;
mod connection;
mod logging_backend;
mod sockets;
mod statistics;
mod storage;

#[cfg(target_os = "android")]
mod audio;

pub mod video_decoder;

use alvr_common::{
    ConnectionState, LifecycleState, ViewParams, dbg_client_core, error,
    glam::{UVec2, Vec2},
    parking_lot::{Mutex, RwLock},
    warn,
};
use alvr_packets::{
    BatteryInfo, ButtonEntry, ClientControlPacket, RealTimeConfig, ReservedClientControlPacket,
    StreamConfig, TrackingData,
};
use alvr_session::CodecType;
use connection::{ConnectionContext, DecoderCallback};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use storage::Config;

pub use logging_backend::init_logging;

pub enum ClientCoreEvent {
    UpdateHudMessage(String),
    StreamingStarted(Box<StreamConfig>),
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
    RealTimeConfig(RealTimeConfig),
}

// Note: this struct may change without breaking network protocol changes
#[derive(Clone)]
pub struct ClientCapabilities {
    pub default_view_resolution: UVec2,
    pub refresh_rates: Vec<f32>,
    pub foveated_encoding: bool,
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
    pub prefer_10bit: bool,
    pub prefer_full_range: bool,
    pub preferred_encoding_gamma: f32,
    pub prefer_hdr: bool,
}

pub struct ClientCoreContext {
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    event_queue: Arc<Mutex<VecDeque<ClientCoreEvent>>>,
    connection_context: Arc<ConnectionContext>,
    connection_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    last_good_global_view_params: Mutex<[ViewParams; 2]>,
}

impl ClientCoreContext {
    pub fn new(capabilities: ClientCapabilities) -> Self {
        dbg_client_core!("Create");

        // Make sure to reset config in case of version compat mismatch.
        if Config::load().protocol_id != alvr_common::protocol_id() {
            // NB: Config::default() sets the current protocol ID
            Config::default().store();
        }

        #[cfg(target_os = "android")]
        {
            dbg_client_core!("Getting permissions");
            alvr_system_info::try_get_permission(alvr_system_info::MICROPHONE_PERMISSION);
            alvr_system_info::set_wifi_lock(true);
        }

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
            last_good_global_view_params: Mutex::new([ViewParams::DUMMY; 2]),
        }
    }

    pub fn resume(&self) {
        dbg_client_core!("resume");

        *self.lifecycle_state.write() = LifecycleState::Resumed;
    }

    pub fn pause(&self) {
        dbg_client_core!("pause");

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
        dbg_client_core!("poll_event");

        self.event_queue.lock().pop_front()
    }

    pub fn send_battery(&self, device_id: u64, gauge_value: f32, is_plugged: bool) {
        dbg_client_core!("send_battery");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::Battery(BatteryInfo {
                    device_id,
                    gauge_value,
                    is_plugged,
                }))
                .ok();
        }
    }

    pub fn send_playspace(&self, area: Option<Vec2>) {
        dbg_client_core!("send_playspace");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::PlayspaceSync(area)).ok();
        }
    }

    pub fn send_active_interaction_profile(&self, device_id: u64, profile_id: u64) {
        dbg_client_core!("send_active_interaction_profile");

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
        dbg_client_core!("send_custom_interaction_profile");

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
        dbg_client_core!("send_buttons");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::Buttons(entries)).ok();
        }
    }

    // These must be in its local space, as if the head pose is in the origin.
    pub fn send_view_params(&self, views: [ViewParams; 2]) {
        dbg_client_core!("send_view_params");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::LocalViewParams(views))
                .ok();
        }
    }

    pub fn send_tracking(&self, data: TrackingData) {
        dbg_client_core!("send_tracking");

        if let Some(sender) = &mut *self.connection_context.tracking_sender.lock() {
            sender.send_header(&data).ok();

            if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
                stats.report_input_acquired(data.poll_timestamp);
            }
        }
    }

    pub fn get_total_prediction_offset(&self) -> Duration {
        dbg_client_core!("get_total_prediction_offset");

        if let Some(stats) = &*self.connection_context.statistics_manager.lock() {
            Duration::min(
                stats.average_total_pipeline_latency(),
                *self.connection_context.max_prediction.read(),
            )
        } else {
            Duration::ZERO
        }
    }

    /// The callback should return true if the frame was successfully submitted to the decoder
    pub fn set_decoder_input_callback(&self, callback: Box<DecoderCallback>) {
        dbg_client_core!("set_decoder_input_callback");

        *self.connection_context.decoder_callback.lock() = Some(callback);

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::RequestIdr).ok();
        }
    }

    pub fn report_frame_decoded(&self, timestamp: Duration) {
        dbg_client_core!("report_frame_decoded");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_frame_decoded(timestamp);
        }
    }

    pub fn report_fatal_decoder_error(&self, error: &str) {
        error!("Fatal decoder error, restarting connection: {error}");

        // The connection loop observes changes on this value
        *self.connection_context.state.write() = ConnectionState::Disconnecting;
    }

    pub fn report_compositor_start(&self, timestamp: Duration) -> [ViewParams; 2] {
        dbg_client_core!("report_compositor_start");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_compositor_start(timestamp);
        }

        let global_view_params_lock = &mut *self.last_good_global_view_params.lock();
        for (ts, params) in &*self.connection_context.global_view_params_queue.lock() {
            if *ts == timestamp {
                *global_view_params_lock = *params;
                break;
            }
        }

        *global_view_params_lock
    }

    pub fn report_submit(&self, timestamp: Duration, vsync_queue: Duration) {
        dbg_client_core!("report_submit");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_submit(timestamp, vsync_queue);

            if let Some(sender) = &mut *self.connection_context.statistics_sender.lock() {
                if let Some(stats) = stats.summary(timestamp) {
                    sender.send_header(&stats).ok();
                } else {
                    warn!("Statistics summary not ready!");
                }
            }
        }
    }
}

impl Drop for ClientCoreContext {
    fn drop(&mut self) {
        dbg_client_core!("Drop");

        *self.lifecycle_state.write() = LifecycleState::ShuttingDown;

        if let Some(thread) = self.connection_thread.lock().take() {
            thread.join().ok();
        }

        #[cfg(target_os = "android")]
        alvr_system_info::set_wifi_lock(false);
    }
}
