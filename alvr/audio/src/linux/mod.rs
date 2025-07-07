pub mod audio;
pub mod microphone;

use std::path::Path;

use alvr_common::{anyhow::Result, error};
use pipewire::{context::Context, main_loop::MainLoop};

struct Terminate;

pub fn try_load_pipewire() -> Result<()> {
    if let Err(e) = probe_pipewire() {
        if matches!(e, pipewire::Error::CreationFailed) {
            error!("Could not initialize PipeWire.");
            if is_currently_under_flatpak() && !is_pipewire_socket_available() {
                error!(
                    "Please visit following page to find help on how fix broken audio on flatpak."
                );
                error!(
                    "https://github.com/alvr-org/ALVR/wiki/Installing-ALVR-and-using-SteamVR-on-Linux-through-Flatpak#failed-to-create-pipewire-errors"
                );
            }
            error!("Make sure PipeWire is installed on your system, running and it's version is at least 0.3.49.
            To retry, please restart SteamVR with ALVR.");
        }
        return Err(e.into());
    }
    Ok(())
}

fn probe_pipewire() -> Result<(), pipewire::Error> {
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    context.connect(None)?;
    Ok(())
}

fn is_currently_under_flatpak() -> bool {
    std::env::var("FLATPAK_ID").is_ok()
}
fn is_pipewire_socket_available() -> bool {
    std::env::var("XDG_RUNTIME_DIR").is_ok_and(|meow| {
        let xdg_runtime_dir = Path::new(&meow);
        let pipewire_path = xdg_runtime_dir.join("pipewire-0");
        xdg_runtime_dir.exists() && pipewire_path.exists()
    })
}
