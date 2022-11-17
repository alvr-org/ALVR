pub fn init_logging() {
    #[cfg(target_os = "android")]
    {
        use crate::CONTROL_CHANNEL_SENDER;
        use alvr_common::log::Level;
        use alvr_events::EventSeverity;
        use std::fmt;

        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("[ALVR NATIVE-RUST]")
                .format(|f, record| {
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
                    fmt::write(f, *record.args())
                })
                .with_min_level(Level::Info),
        );
    }

    alvr_common::set_panic_hook();
}
