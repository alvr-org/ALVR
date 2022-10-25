use crate::{AlvrCodec, AlvrEvent, EVENT_QUEUE, STATISTICS_MANAGER};
use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, RelaxedAtomic};
use alvr_session::{CodecType, MediacodecDataType};
use std::{collections::VecDeque, ffi::c_char, ptr, time::Duration};

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

struct ReconstructedNal {
    timestamp: Duration,
    data: Vec<u8>,
}

pub static EXTERNAL_DECODER: RelaxedAtomic = RelaxedAtomic::new(false);
static NAL_QUEUE: Lazy<Mutex<VecDeque<ReconstructedNal>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static LAST_ENQUEUED_TIMESTAMPS: Lazy<Mutex<VecDeque<Duration>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

pub extern "C" fn create_decoder(buffer: *const c_char, length: i32) {
    let mut csd_0 = vec![0; length as _];
    unsafe { ptr::copy_nonoverlapping(buffer, csd_0.as_mut_ptr() as _, length as _) };

    let config = DECODER_INIT_CONFIG.lock();

    if EXTERNAL_DECODER.value() {
        // duration == 0 is the flag to identify the config NALS
        NAL_QUEUE.lock().push_back(ReconstructedNal {
            timestamp: Duration::ZERO,
            data: csd_0,
        });
        EVENT_QUEUE.lock().push_back(AlvrEvent::CreateDecoder {
            codec: if matches!(config.codec, CodecType::H264) {
                AlvrCodec::H264
            } else {
                AlvrCodec::H265
            },
        });
    } else {
        #[cfg(target_os = "android")]
        if DECODER_ENQUEUER.lock().is_none() {
            let (enqueuer, dequeuer) =
                crate::platform::video_decoder_split(config.clone(), &csd_0, |target_timestamp| {
                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_frame_decoded(target_timestamp);
                    }
                })
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

pub extern "C" fn push_nal(buffer: *const c_char, length: i32, timestamp_ns: u64) {
    let timestamp = Duration::from_nanos(timestamp_ns);

    {
        let mut timestamps_lock = LAST_ENQUEUED_TIMESTAMPS.lock();

        timestamps_lock.push_back(timestamp);
        if timestamps_lock.len() > 20 {
            timestamps_lock.pop_front();
        }
    }

    let mut data = vec![0; length as _];
    unsafe { ptr::copy_nonoverlapping(buffer, data.as_mut_ptr() as _, length as _) }

    if EXTERNAL_DECODER.value() {
        NAL_QUEUE
            .lock()
            .push_back(ReconstructedNal { timestamp, data });
        EVENT_QUEUE.lock().push_back(AlvrEvent::NalReady);
    } else {
        #[cfg(target_os = "android")]
        if let Some(decoder) = &*DECODER_ENQUEUER.lock() {
            show_err(decoder.push_frame_nal(timestamp, &data, Duration::from_millis(500)));
        } else if let Some(sender) = &*crate::CONTROL_CHANNEL_SENDER.lock() {
            sender
                .send(alvr_sockets::ClientControlPacket::RequestIdr)
                .ok();
        }
    }
}

/// Call only with internal decoder (Android only)
/// Returns frame timestamp in nanoseconds or -1 if no frame available. Returns an AHardwareBuffer
/// from out_buffer.
#[cfg(target_os = "android")]
#[allow(unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn alvr_get_frame(out_buffer: *mut *mut std::ffi::c_void) -> i64 {
    let timestamp = if let Some(decoder) = &mut *DECODER_DEQUEUER.lock() {
        if let Some(crate::platform::android::DequeuedFrame {
            timestamp,
            buffer_ptr,
        }) = decoder.dequeue_frame()
        {
            *out_buffer = buffer_ptr;

            Some(timestamp)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(timestamp) = timestamp {
        if !LAST_ENQUEUED_TIMESTAMPS.lock().contains(&timestamp) {
            error!("Detected late decoder, recreating decoder...");
            *DECODER_ENQUEUER.lock() = None;
            *DECODER_DEQUEUER.lock() = None;
        }

        if let Some(stats) = &mut *crate::STATISTICS_MANAGER.lock() {
            stats.report_compositor_start(timestamp);
        }

        timestamp.as_nanos() as i64
    } else {
        -1
    }
}

/// Call only with external decoder
/// Returns the number of bytes of the next nal, or 0 if there are no nals ready.
/// If out_nal or out_timestamp_ns is null, no nal is dequeued. Use to get the nal allocation size.
/// Returns out_timestamp_ns == 0 if config NAL.
#[no_mangle]
pub extern "C" fn alvr_poll_nal(out_nal: *mut c_char, out_timestamp_ns: *mut u64) -> u64 {
    let mut queue_lock = NAL_QUEUE.lock();
    if let Some(ReconstructedNal { timestamp, data }) = queue_lock.pop_front() {
        let nal_size = data.len();
        if !out_nal.is_null() && !out_timestamp_ns.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr(), out_nal as _, nal_size);
                *out_timestamp_ns = timestamp.as_nanos() as _;
            }
        } else {
            queue_lock.push_front(ReconstructedNal { timestamp, data })
        }

        nal_size as u64
    } else {
        0
    }
}
