pub mod windows_steamvr {
    pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn start_steamvr() {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(["/C", "start", "steam://rungameid/250820"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok();
    }

    pub fn kill_process(pid: u32) {
        use std::os::windows::process::CommandExt;
        Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok();
    }
}
