mod dashboard;
mod tabs;
mod theme;

pub use dashboard::{Dashboard, DashboardEvent};

use alvr_sockets::{AudioDevicesList, ClientListAction, PathSegment};

// Abstraction layer for command execution/data retrieval. The backend could be in the server or in
// the same process (the launcher before cnnecting to the server).
pub struct DashboardDataInterfce {
    pub set_session_cb: Box<dyn Fn(Vec<PathSegment>, &str)>,
    pub execute_script_cb: Box<dyn Fn(&str) -> Option<String>>,
    pub get_gpu_name_cb: Box<dyn Fn() -> String>,
    pub get_audio_devices_list_cb: Box<dyn Fn() -> AudioDevicesList>,
    pub update_client_list: Box<dyn Fn(String, ClientListAction)>,
}

impl DashboardDataInterfce {
    pub fn set_single_value(&mut self, key_path: Vec<PathSegment>, value: &str) {
        (self.set_session_cb)(key_path, value);
    }

    pub fn execute_script(&self, code: &str) -> Option<String> {
        (self.execute_script_cb)(code)
    }

    pub fn get_gpu_name(&self) -> String {
        (self.get_gpu_name_cb)()
    }

    pub fn get_audio_devices_list(&self) -> AudioDevicesList {
        (self.get_audio_devices_list_cb)()
    }

    pub fn update_client_list(&self, hostname: String, action: ClientListAction) {
        (self.update_client_list)(hostname, action)
    }
}
