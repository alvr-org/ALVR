use crate::{
    platform, AlvrCodec, AlvrEvent, CONTROL_CHANNEL_SENDER, DISCONNECT_NOTIFIER, EVENT_QUEUE,
    STATISTICS_MANAGER,
};
use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, prelude::*, RelaxedAtomic};
use alvr_session::{CodecType, MediacodecDataType};
use alvr_sockets::ClientControlPacket;
use std::{
    collections::VecDeque,
    ffi::c_void,
    os::raw::c_char,
    ptr, thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "android")]
use crate::platform::{DecoderDequeuedData, VideoDecoderDequeuer, VideoDecoderEnqueuer};

pub struct DecoderInitConfig {
    pub codec: CodecType,
    pub options: Vec<(String, MediacodecDataType)>,
}

pub static DECODER_INIT_CONFIG: Lazy<Mutex<DecoderInitConfig>> = Lazy::new(|| {
    Mutex::new(DecoderInitConfig {
        codec: CodecType::H264,
        options: vec![],
    })
});
#[cfg(target_os = "android")]
pub static DECODER_ENQUEUER: Lazy<Mutex<Option<VideoDecoderEnqueuer>>> =
    Lazy::new(|| Mutex::new(None));
#[cfg(target_os = "android")]
pub static DECODER_DEQUEUER: Lazy<Mutex<Option<VideoDecoderDequeuer>>> =
    Lazy::new(|| Mutex::new(None));

pub static EXTERNAL_DECODER: RelaxedAtomic = RelaxedAtomic::new(false);
static NAL_QUEUE: Lazy<Mutex<VecDeque<(Duration, Vec<u8>)>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static LAST_ENQUEUED_TIMESTAMPS: Lazy<Mutex<VecDeque<Duration>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

pub extern "C" fn create_decoder(buffer: *const c_char, length: i32) {
    let mut csd_0 = vec![0; length as _];
    unsafe { ptr::copy_nonoverlapping(buffer, csd_0.as_mut_ptr() as _, length as _) };

    let config = DECODER_INIT_CONFIG.lock();

    if EXTERNAL_DECODER.value() {
        // duration == 0 is the flag to identify the config NALS
        NAL_QUEUE.lock().push_back((Duration::ZERO, csd_0));
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
                platform::video_decoder_split(config.codec, &csd_0, &config.options).unwrap();

            *DECODER_ENQUEUER.lock() = Some(enqueuer);
            *DECODER_DEQUEUER.lock() = Some(dequeuer);

            if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                sender.send(ClientControlPacket::RequestIdr).ok();
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

    let mut nal_buffer = vec![0; length as _];
    unsafe { ptr::copy_nonoverlapping(buffer, nal_buffer.as_mut_ptr() as _, length as _) }

    if EXTERNAL_DECODER.value() {
        NAL_QUEUE.lock().push_back((timestamp, nal_buffer));
        EVENT_QUEUE.lock().push_back(AlvrEvent::NalReady);
    } else {
        #[cfg(target_os = "android")]
        if let Some(decoder) = &*DECODER_ENQUEUER.lock() {
            show_err(decoder.push_frame_nal(timestamp, &nal_buffer, Duration::from_millis(500)));
        } else if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            sender.send(ClientControlPacket::RequestIdr).ok();
        }
    }
}

/// Call only with internal decoder
/// Returns frame timestamp in nanoseconds or -1 if no frame available. Returns an AHardwareBuffer
/// from out_buffer.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_wait_for_frame(out_buffer: *mut *mut c_void) -> i64 {
    let timestamp = if let Some(decoder) = &*DECODER_DEQUEUER.lock() {
        // Note on frame pacing: sometines there could be late frames stored inside the decoder,
        // which are gradually drained by polling two frames per frame. But sometimes a frame could
        // be received earlier than usual because of network jitter. In this case, if we polled the
        // second frame immediately, the next frame would probably be late. To mitigate this
        // scenario, a 5ms delay measurement is used to decide if to poll the second frame or not.
        // todo: remove the 5ms "magic number" and implement proper phase sync measuring network
        // jitter variance.
        let start_instant = Instant::now();
        match decoder.dequeue_frame(Duration::from_millis(50), Duration::from_millis(100)) {
            Ok(DecoderDequeuedData::Frame {
                buffer_ptr,
                timestamp,
            }) => {
                if Instant::now() - start_instant < Duration::from_millis(5) {
                    debug!("Try draining extra decoder frame");
                    match decoder
                        .dequeue_frame(Duration::from_micros(1), Duration::from_millis(100))
                    {
                        Ok(DecoderDequeuedData::Frame {
                            buffer_ptr,
                            timestamp,
                        }) => {
                            *out_buffer = buffer_ptr;
                            Some(timestamp)
                        }
                        Ok(_) => {
                            // Note: data from first dequeue!
                            *out_buffer = buffer_ptr;
                            Some(timestamp)
                        }
                        Err(e) => {
                            error!("Error while decoder dequeue (2nd time): {e}");
                            DISCONNECT_NOTIFIER.notify_waiters();

                            None
                        }
                    }
                } else {
                    *out_buffer = buffer_ptr;
                    Some(timestamp)
                }
            }
            Ok(data) => {
                info!("Decoder: no frame dequeued. {data:?}");

                None
            }
            Err(e) => {
                error!("Error while decoder dequeue: {e}");
                DISCONNECT_NOTIFIER.notify_waiters();

                None
            }
        }
    } else {
        thread::sleep(Duration::from_millis(5));
        None
    };

    if let Some(timestamp) = timestamp {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_decoded(timestamp);
        }

        if !LAST_ENQUEUED_TIMESTAMPS.lock().contains(&timestamp) {
            error!("Detected late decoder, disconnecting...");
            DISCONNECT_NOTIFIER.notify_waiters();
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
    if let Some((timestamp, nal)) = queue_lock.pop_front() {
        let nal_size = nal.len();
        if !out_nal.is_null() && !out_timestamp_ns.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(nal.as_ptr(), out_nal as _, nal_size);
                *out_timestamp_ns = timestamp.as_nanos() as _;
            }
        } else {
            queue_lock.push_front((timestamp, nal))
        }

        nal_size as u64
    } else {
        0
    }
}
