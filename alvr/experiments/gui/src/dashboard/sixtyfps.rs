use alvr_common::ServerEvent;
use alvr_session::SessionDesc;

sixtyfps::include_modules!();

pub struct Dashboard {
    inner: DashboardWindow,
}

impl Dashboard {
    pub fn new(session: SessionDesc) -> Self {
        Self {
            inner: DashboardWindow::new(),
        }
    }

    pub fn run(&self, mut event_handler: impl FnMut(String) -> String) {
        self.inner.run();
    }

    pub fn report_event(&self, event: ServerEvent) {}
}
