pub mod audio;
pub mod microphone;

use alvr_common::{anyhow::Result, error};
use pipewire::main_loop::MainLoop;

struct Terminate;

pub fn try_load_pipewire() -> Result<()> {
    if let Err(e) = MainLoop::new(None) {
        if matches!(e, pipewire::Error::CreationFailed) {
            error!(
                "Could not initialize PipeWire. 
        Make sure PipeWire is installed on your system and it's version is at least 0.3.49.
        To retry, please restart SteamVR with ALVR."
            );
        }
        return Err(e.into());
    }
    Ok(())
}
