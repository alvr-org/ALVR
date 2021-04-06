pub mod data;
pub mod logging;

#[cfg(any(windows, target_os = "linux", target_os = "android"))]
pub mod audio;
#[cfg(any(windows, target_os = "linux", target_os = "android"))]
pub mod sockets;

#[cfg(any(windows, target_os = "linux"))]
pub mod commands;
#[cfg(any(windows, target_os = "linux"))]
pub mod ffmpeg;
#[cfg(any(windows, target_os = "linux"))]
pub mod graphics;

pub mod prelude {
    pub use crate::{
        fmt_e,
        logging::{log_event, Event, StrResult},
        trace_err, trace_none, trace_str,
    };
    pub use log::{debug, error, info, warn};
}

////////////////////////////////////////////////////////

#[cfg(any(windows, target_os = "linux", target_os = "android"))]
mod util {
    use crate::prelude::*;
    use std::future::Future;
    use tokio::{sync::oneshot, task};

    // Tokio tasks are not cancelable. This function awaits a cancelable task.
    pub async fn spawn_cancelable(
        future: impl Future<Output = StrResult> + Send + 'static,
    ) -> StrResult {
        // this channel is actually never used. cancel_receiver will be notified when _cancel_sender
        // is dropped
        let (_cancel_sender, cancel_receiver) = oneshot::channel::<()>();

        trace_err!(
            task::spawn(async {
                tokio::select! {
                    res = future => res,
                    _ = cancel_receiver => Ok(()),
                }
            })
            .await
        )?
    }
}
#[cfg(any(windows, target_os = "linux", target_os = "android"))]
pub use util::*;
