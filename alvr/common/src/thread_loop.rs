use crate::*;
use std::sync::{atomic::*, Arc};
use std::thread::{self, JoinHandle};

pub struct ThreadLoop {
    join_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl ThreadLoop {
    pub fn request_stop(&mut self) {
        self.running.store(false, Ordering::Relaxed)
    }
}

impl Drop for ThreadLoop {
    fn drop(&mut self) {
        self.request_stop();
        self.join_handle.take().map(|h| h.join());
    }
}

pub fn spawn(name: &str, mut loop_body: impl FnMut() + Send + 'static) -> StrResult<ThreadLoop> {
    let running = Arc::new(AtomicBool::new(true));

    let join_handle = Some(trace_err!(thread::Builder::new().name(name.into()).spawn({
        let running = running.clone();
        move || {
            while running.load(Ordering::Relaxed) {
                loop_body()
            }
        }
    }))?);

    Ok(ThreadLoop {
        join_handle,
        running,
    })
}
