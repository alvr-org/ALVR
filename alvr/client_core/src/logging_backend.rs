use crate::CONTROL_CHANNEL_SENDER;
use alvr_common::log::{Level, Record};
use alvr_events::EventSeverity;

pub fn init_logging() {
    fn send_log(record: &Record) {
        if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            let level = match record.level() {
                Level::Error => EventSeverity::Error,
                Level::Warn => EventSeverity::Warning,
                Level::Info => EventSeverity::Info,
                _ => EventSeverity::Debug,
            };

            sender
                .send(alvr_sockets::ClientControlPacket::Log {
                    level,
                    message: format!("{}", record.args()),
                })
                .ok();
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
                .with_min_level(Level::Info),
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
