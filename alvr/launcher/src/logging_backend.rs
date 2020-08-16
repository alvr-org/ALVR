use fern::Dispatch;
use log::{Level, LevelFilter};
use msgbox::IconType;

const MSGBOX_TITLE: &str = "ALVR launcher";

pub fn init_logging() {
    let mut log_dispatch = Dispatch::new().format(move |out, message, record| {
        match record.level() {
            Level::Info => msgbox::create(MSGBOX_TITLE, &message.to_string(), IconType::Info),
            Level::Error => msgbox::create(MSGBOX_TITLE, &message.to_string(), IconType::Error),
            // note: msgbox does not have a warning icon
            _ => msgbox::create(MSGBOX_TITLE, &message.to_string(), IconType::None),
        }

        out.finish(format_args!("{}", message));
    });

    if cfg!(debug_assertions) {
        log_dispatch = log_dispatch
            .level(LevelFilter::Debug)
            .chain(std::io::stdout());
    } else {
        log_dispatch = log_dispatch.level(LevelFilter::Info);
    }

    log_dispatch.apply().unwrap();
}
