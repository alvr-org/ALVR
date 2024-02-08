use crate::{ClientCoreEvent, EVENT_QUEUE};
use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, RelaxedAtomic};
use alvr_packets::DecoderInitializationConfig;
use alvr_session::{CodecType, MediacodecDataType};
use std::time::Duration;

#[derive(Clone)]
pub struct DecoderInitConfig {
    pub codec: CodecType,
    pub force_software_decoder: bool,
    pub max_buffering_frames: f32,
    pub buffering_history_weight: f32,
    pub options: Vec<(String, MediacodecDataType)>,
}

pub static DECODER_INIT_CONFIG: Lazy<Mutex<DecoderInitConfig>> = Lazy::new(|| {
    Mutex::new(DecoderInitConfig {
        codec: CodecType::H264,
        force_software_decoder: false,
        max_buffering_frames: 1.0,
        buffering_history_weight: 0.9,
        options: vec![],
    })
});
#[cfg(target_os = "android")]
pub static DECODER_SINK: alvr_common::OptLazy<crate::platform::VideoDecoderSink> =
    alvr_common::lazy_mut_none();
#[cfg(target_os = "android")]
pub static DECODER_SOURCE: alvr_common::OptLazy<crate::platform::VideoDecoderSource> =
    alvr_common::lazy_mut_none();

pub static EXTERNAL_DECODER: RelaxedAtomic = RelaxedAtomic::new(false);

pub fn maybe_create_decoder(
    lazy_config: DecoderInitializationConfig,
    force_software_decoder: bool,
) {
    let mut config = DECODER_INIT_CONFIG.lock();
    config.codec = lazy_config.codec;
    config.force_software_decoder = force_software_decoder;

    if EXTERNAL_DECODER.value() {
        EVENT_QUEUE
            .lock()
            .push_back(ClientCoreEvent::DecoderConfig {
                codec: config.codec,
                config_nal: lazy_config.config_buffer,
            });
    } else {
        #[cfg(target_os = "android")]
        if DECODER_SINK.lock().is_none() {
            let (enqueuer, dequeuer) = crate::platform::video_decoder_split(
                config.clone(),
                lazy_config.config_buffer,
                |target_timestamp| {
                    if let Some(stats) = &mut *crate::STATISTICS_MANAGER.lock() {
                        stats.report_frame_decoded(target_timestamp);
                    }
                },
            )
            .unwrap();

            *DECODER_SINK.lock() = Some(enqueuer);
            *DECODER_SOURCE.lock() = Some(dequeuer);

            if let Some(sender) = &mut *crate::connection::CONTROL_SENDER.lock() {
                sender
                    .send(&alvr_packets::ClientControlPacket::RequestIdr)
                    .ok();
            }
        }
    }
}

// return: frame has been successfully enqueued
pub fn push_nal(timestamp: Duration, nal: &[u8]) -> bool {
    if EXTERNAL_DECODER.value() {
        EVENT_QUEUE.lock().push_back(ClientCoreEvent::FrameReady {
            timestamp,
            nal: nal.to_vec(),
        });
        true
    } else {
        #[cfg(target_os = "android")]
        if let Some(decoder) = &mut *DECODER_SINK.lock() {
            matches!(
                alvr_common::show_err(decoder.push_frame_nal(timestamp, nal)),
                Some(true)
            )
        } else {
            false
        }
        #[cfg(not(target_os = "android"))]
        false
    }
}

/// Call only with internal decoder (Android only)
/// If a frame is available, return the timestamp and the AHardwareBuffer.
pub fn get_frame() -> Option<(Duration, *mut std::ffi::c_void)> {
    #[cfg(target_os = "android")]
    if let Some(decoder) = &mut *DECODER_SOURCE.lock() {
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
