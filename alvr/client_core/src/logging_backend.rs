use alvr_common::{
    log::{Level, Record},
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    DebugGroupsConfig, LogSeverity, OptLazy,
};
use alvr_packets::ClientControlPacket;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

const LOG_REPEAT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct LogMirrorData {
    pub sender: mpsc::Sender<ClientControlPacket>,
    pub filter_level: LogSeverity,
    pub debug_groups_config: DebugGroupsConfig,
}

pub static LOG_CHANNEL_SENDER: OptLazy<LogMirrorData> = alvr_common::lazy_mut_none();

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
