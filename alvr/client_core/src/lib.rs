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

pub use decoder::get_frame;
pub use logging_backend::init_logging;
#[cfg(target_os = "android")]
pub use platform::try_get_permission;

use alvr_common::{
    error,
    glam::{UVec2, Vec2},
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    Fov, LazyMutOpt, RelaxedAtomic,
};
use alvr_packets::{BatteryPacket, ButtonEntry, ClientControlPacket, Tracking, ViewsConfig};
use alvr_session::{CodecType, Settings};
use connection::{CONTROL_SENDER, STATISTICS_SENDER, TRACKING_SENDER};
use decoder::EXTERNAL_DECODER;
use serde::{Deserialize, Serialize};
use statistics::StatisticsManager;
use std::{
    collections::VecDeque,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use storage::Config;

static STATISTICS_MANAGER: LazyMutOpt<StatisticsManager> = alvr_common::lazy_mut_none();

static EVENT_QUEUE: Lazy<Mutex<VecDeque<ClientCoreEvent>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static IS_ALIVE: RelaxedAtomic = RelaxedAtomic::new(true);
static IS_RESUMED: RelaxedAtomic = RelaxedAtomic::new(false);
static IS_STREAMING: Lazy<Arc<RelaxedAtomic>> = Lazy::new(|| Arc::new(RelaxedAtomic::new(false)));

static CONNECTION_THREAD: LazyMutOpt<JoinHandle<()>> = alvr_common::lazy_mut_none();

#[derive(Serialize, Deserialize)]
pub enum ClientCoreEvent {
    UpdateHudMessage(String),
    StreamingStarted {
        view_resolution: UVec2,
        refresh_rate_hint: f32,
        settings: Box<Settings>,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
    CreateDecoder {
        codec: CodecType,
        config_nal: Vec<u8>,
    },
    FrameReady {
        timestamp: Duration,
        nal: Vec<u8>,
    },
}

pub fn device_model() -> String {
    platform::device_model()
}

pub fn manufacturer_name() -> String {
    platform::manufacturer_name()
}

pub fn initialize(
    recommended_view_resolution: UVec2,
    supported_refresh_rates: Vec<f32>,
    external_decoder: bool,
) {
    logging_backend::init_logging();

    // Make sure to reset config in case of version compat mismatch.
    if Config::load().protocol_id != alvr_common::protocol_id() {
        // NB: Config::default() sets the current protocol ID
        Config::default().store();
    }

    #[cfg(target_os = "android")]
    platform::try_get_permission(platform::MICROPHONE_PERMISSION);
    #[cfg(target_os = "android")]
    platform::acquire_wifi_lock();

    IS_ALIVE.set(true);
    EXTERNAL_DECODER.set(external_decoder);

    *CONNECTION_THREAD.lock() = Some(thread::spawn(move || {
        connection::connection_lifecycle_loop(recommended_view_resolution, supported_refresh_rates)
    }));
}

pub fn destroy() {
    IS_ALIVE.set(false);

    if let Some(thread) = CONNECTION_THREAD.lock().take() {
        thread.join().ok();
    }

    #[cfg(target_os = "android")]
    platform::release_wifi_lock();
}

pub fn resume() {
    IS_RESUMED.set(true);
}

pub fn pause() {
    IS_RESUMED.set(false);
}

pub fn poll_event() -> Option<ClientCoreEvent> {
    EVENT_QUEUE.lock().pop_front()
}

pub fn send_views_config(fov: [Fov; 2], ipd_m: f32) {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender
            .send(&ClientControlPacket::ViewsConfig(ViewsConfig {
                fov,
                ipd_m,
            }))
            .ok();
    }
}

pub fn send_battery(device_id: u64, gauge_value: f32, is_plugged: bool) {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender
            .send(&ClientControlPacket::Battery(BatteryPacket {
                device_id,
                gauge_value,
                is_plugged,
            }))
            .ok();
    }
}

pub fn send_playspace(area: Option<Vec2>) {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender.send(&ClientControlPacket::PlayspaceSync(area)).ok();
    }
}

pub fn send_active_interaction_profile(device_id: u64, profile_id: u64) {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender
            .send(&ClientControlPacket::ActiveInteractionProfile {
                device_id,
                profile_id,
            })
            .ok();
    }
}

pub fn send_buttons(entries: Vec<ButtonEntry>) {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender.send(&ClientControlPacket::Buttons(entries)).ok();
    }
}

pub fn send_tracking(tracking: Tracking) {
    if let Some(sender) = &mut *TRACKING_SENDER.lock() {
        sender.send_header(&tracking).ok();

        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_input_acquired(tracking.target_timestamp);
        }
    }
}

pub fn get_head_prediction_offset() -> Duration {
    if let Some(stats) = &*STATISTICS_MANAGER.lock() {
        stats.average_total_pipeline_latency()
    } else {
        Duration::ZERO
    }
}

pub fn get_tracker_prediction_offset() -> Duration {
    if let Some(stats) = &*STATISTICS_MANAGER.lock() {
        stats.tracker_prediction_offset()
    } else {
        Duration::ZERO
    }
}

pub fn report_submit(target_timestamp: Duration, vsync_queue: Duration) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_submit(target_timestamp, vsync_queue);

        if let Some(sender) = &mut *STATISTICS_SENDER.lock() {
            if let Some(stats) = stats.summary(target_timestamp) {
                sender.send_header(&stats).ok();
            } else {
                error!("Statistics summary not ready!");
            }
        }
    }
}

/// Call only with external decoder
pub fn request_idr() {
    if let Some(sender) = &mut *CONTROL_SENDER.lock() {
        sender.send(&ClientControlPacket::RequestIdr).ok();
    }
}

/// Call only with external decoder
pub fn report_frame_decoded(target_timestamp: Duration) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_frame_decoded(target_timestamp);
    }
}

/// Call only with external decoder
pub fn report_compositor_start(target_timestamp: Duration) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_compositor_start(target_timestamp);
    }
}
