use crate::CONTROL_CHANNEL_SENDER;
use alvr_common::{
    log::{Level, Record},
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    LogSeverity,
};
use alvr_packets::ClientControlPacket;
use std::time::{Duration, Instant};

const LOG_REPEAT_TIMEOUT: Duration = Duration::from_secs(1);

struct RepeatedLogEvent {
    message: String,
    repetition_times: usize,
    initial_timestamp: Instant,
}

static LAST_LOG_EVENT: Lazy<Mutex<RepeatedLogEvent>> = Lazy::new(|| {
    Mutex::new(RepeatedLogEvent {
        message: "".into(),
        repetition_times: 0,
        initial_timestamp: Instant::now(),
    })
});

pub fn init_logging() {
    fn send_log(record: &Record) {
        if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            let level = match record.level() {
                Level::Error => LogSeverity::Error,
                Level::Warn => LogSeverity::Warning,
                Level::Info => LogSeverity::Info,
                _ => LogSeverity::Debug,
            };

            let message = format!("{}", record.args());

            let mut last_log_event_lock = LAST_LOG_EVENT.lock();

            if last_log_event_lock.message == message
                && last_log_event_lock.initial_timestamp + LOG_REPEAT_TIMEOUT > Instant::now()
            {
                last_log_event_lock.repetition_times += 1;
            } else {
                if last_log_event_lock.repetition_times > 1 {
                    sender
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

                sender
                    .send(ClientControlPacket::Log { level, message })
                    .ok();
            }
        }
    }

    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("[ALVR NATIVE-RUST]")
                .format(|f, record| {
                    send_log(&record);
                    std::fmt::write(f, *record.args())
                })
                .with_max_level(alvr_common::log::LevelFilter::Info),
        );
    }
    #[cfg(not(target_os = "android"))]
    {
        use std::io::Write;
        env_logger::builder()
            .format(|f, record| {
                send_log(record);
                writeln!(f, "{}", record.args())
            })
            .try_init()
            .ok();
    }

    alvr_common::set_panic_hook();
}
