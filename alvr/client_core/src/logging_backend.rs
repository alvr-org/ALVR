use alvr_common::{
    DebugGroupsConfig, LogSeverity,
    log::{Level, Record},
    parking_lot::Mutex,
};
use alvr_packets::ClientControlPacket;
use std::{
    sync::{LazyLock, mpsc},
    time::{Duration, Instant},
};

const LOG_REPEAT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct LogMirrorData {
    pub sender: mpsc::Sender<ClientControlPacket>,
    pub filter_level: LogSeverity,
    pub debug_groups_config: DebugGroupsConfig,
}

pub static LOG_CHANNEL_SENDER: Mutex<Option<LogMirrorData>> = Mutex::new(None);

struct RepeatedLogEvent {
    message: String,
    repetition_times: usize,
    initial_timestamp: Instant,
}

static LAST_LOG_EVENT: LazyLock<Mutex<RepeatedLogEvent>> = LazyLock::new(|| {
    Mutex::new(RepeatedLogEvent {
        message: "".into(),
        repetition_times: 0,
        initial_timestamp: Instant::now(),
    })
});

/// Writes a diagnostic line straight to logcat, bypassing the log channel, severity filter and
/// debug-group filtering that `init_logging` applies. Used by the alpha stream instrumentation so
/// the messages are guaranteed to be visible via `adb logcat` regardless of session settings.
#[cfg(target_os = "android")]
pub fn alpha_debug_log(message: &str) {
    use std::ffi::CString;

    unsafe extern "C" {
        fn __android_log_write(prio: i32, tag: *const std::ffi::c_char, text: *const std::ffi::c_char) -> i32;
    }

    const ANDROID_LOG_INFO: i32 = 4;

    if let (Ok(tag), Ok(text)) = (CString::new("ALPHADBG"), CString::new(message)) {
        unsafe { __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), text.as_ptr()) };
    }
}

#[cfg(not(target_os = "android"))]
pub fn alpha_debug_log(message: &str) {
    println!("ALPHADBG {message}");
}

pub fn init_logging() {
    fn send_log(record: &Record) -> bool {
        let Some(data) = &*LOG_CHANNEL_SENDER.lock() else {
            // if channel has not been setup, always print everything to stdout
            // todo: the client debug groups settings should be moved client side when feasible
            return true;
        };

        let level = match record.level() {
            Level::Error => LogSeverity::Error,
            Level::Warn => LogSeverity::Warning,
            Level::Info => LogSeverity::Info,
            Level::Debug | Level::Trace => LogSeverity::Debug,
        };
        if level < data.filter_level {
            return false;
        }

        let message = format!("{}", record.args());

        if !alvr_common::filter_debug_groups(&message, &data.debug_groups_config) {
            return false;
        }

        let mut last_log_event_lock = LAST_LOG_EVENT.lock();

        if last_log_event_lock.message == message
            && last_log_event_lock.initial_timestamp + LOG_REPEAT_TIMEOUT > Instant::now()
        {
            last_log_event_lock.repetition_times += 1;
        } else {
            if last_log_event_lock.repetition_times > 1 {
                data.sender
                    .send(ClientControlPacket::Log {
                        level: LogSeverity::Info,
                        message: format!(
                            "Last log line repeated {} times",
                            last_log_event_lock.repetition_times
                        ),
                    })
                    .ok();
            }

            *last_log_event_lock = RepeatedLogEvent {
                message: message.clone(),
                repetition_times: 1,
                initial_timestamp: Instant::now(),
            };

            data.sender
                .send(ClientControlPacket::Log { level, message })
                .ok();
        }

        true
    }

    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("[ALVR NATIVE-RUST]")
                .format(|f, record| {
                    if send_log(record) {
                        writeln!(f, "{}", record.args())
                    } else {
                        Ok(())
                    }
                })
                .with_max_level(alvr_common::log::LevelFilter::Info),
        );
    }
    #[cfg(not(target_os = "android"))]
    {
        use std::io::Write;
        env_logger::builder()
            .format(|f, record| {
                if send_log(record) {
                    writeln!(f, "{}", record.args())
                } else {
                    Ok(())
                }
            })
            .try_init()
            .ok();
    }

    alvr_common::set_panic_hook();
}
