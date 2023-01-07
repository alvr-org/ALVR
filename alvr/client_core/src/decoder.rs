use crate::{ClientCoreEvent, EVENT_QUEUE};
use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, RelaxedAtomic};
use alvr_session::{CodecType, MediacodecDataType};
use bytes::BytesMut;
use std::time::Duration;

#[cfg(target_os = "android")]
use alvr_common::prelude::*;

#[derive(Clone)]
pub struct DecoderInitConfig {
    pub codec: CodecType,
    pub max_buffering_frames: f32,
    pub buffering_history_weight: f32,
    pub options: Vec<(String, MediacodecDataType)>,
}

pub static DECODER_INIT_CONFIG: Lazy<Mutex<DecoderInitConfig>> = Lazy::new(|| {
    Mutex::new(DecoderInitConfig {
        codec: CodecType::H264,
        max_buffering_frames: 1.0,
        buffering_history_weight: 0.9,
        options: vec![],
    })
});
#[cfg(target_os = "android")]
pub static DECODER_ENQUEUER: Lazy<Mutex<Option<crate::platform::VideoDecoderEnqueuer>>> =
    Lazy::new(|| Mutex::new(None));
#[cfg(target_os = "android")]
pub static DECODER_DEQUEUER: Lazy<Mutex<Option<crate::platform::VideoDecoderDequeuer>>> =
    Lazy::new(|| Mutex::new(None));

pub static EXTERNAL_DECODER: RelaxedAtomic = RelaxedAtomic::new(false);

pub fn create_decoder(config_nal: Vec<u8>) {
    let config = DECODER_INIT_CONFIG.lock();

    if EXTERNAL_DECODER.value() {
        EVENT_QUEUE
            .lock()
            .push_back(ClientCoreEvent::CreateDecoder {
                codec: config.codec,
                config_nal,
            });
    } else {
        #[cfg(target_os = "android")]
        if DECODER_ENQUEUER.lock().is_none() {
            let (enqueuer, dequeuer) = crate::platform::video_decoder_split(
                config.clone(),
                config_nal,
                |target_timestamp| {
                    if let Some(stats) = &mut *crate::STATISTICS_MANAGER.lock() {
                        stats.report_frame_decoded(target_timestamp);
                    }
                },
            )
            .unwrap();

            *DECODER_ENQUEUER.lock() = Some(enqueuer);
            *DECODER_DEQUEUER.lock() = Some(dequeuer);

            if let Some(sender) = &*crate::CONTROL_CHANNEL_SENDER.lock() {
                sender
                    .send(alvr_sockets::ClientControlPacket::RequestIdr)
                    .ok();
            }
        }
    }
}

pub fn push_nal(buffer: BytesMut, timestamp_ns: u64) {
    let timestamp = Duration::from_nanos(timestamp_ns);

    if EXTERNAL_DECODER.value() {
        let mut nal = vec![0; buffer.len() as _];
        nal.copy_from_slice(&buffer);
        EVENT_QUEUE
            .lock()
            .push_back(ClientCoreEvent::FrameReady { timestamp, nal });
    } else {
        #[cfg(target_os = "android")]
        if let Some(decoder) = &*DECODER_ENQUEUER.lock() {
            show_err(decoder.push_frame_nal(timestamp, buffer, Duration::from_millis(500)));
        } else if let Some(sender) = &*crate::CONTROL_CHANNEL_SENDER.lock() {
            sender
                .send(alvr_sockets::ClientControlPacket::RequestIdr)
                .ok();
        }
    }
}

/// Call only with internal decoder (Android only)
/// If a frame is available, return the timestamp and the AHardwareBuffer.
pub fn get_frame() -> Option<(Duration, *mut std::ffi::c_void)> {
    #[cfg(target_os = "android")]
    if let Some(decoder) = &mut *DECODER_DEQUEUER.lock() {
        if let Some((timestamp, buffer_ptr)) = decoder.dequeue_frame() {
            if let Some(stats) = &mut *crate::STATISTICS_MANAGER.lock() {
                stats.report_compositor_start(timestamp);
            }

            Some((timestamp, buffer_ptr))
        } else {
            None
        }
    } else {
        None
    }
    #[cfg(not(target_os = "android"))]
    None
}
