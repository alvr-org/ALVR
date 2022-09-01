use alvr_common::{prelude::*, send_launcher_packet, LauncherMessages, LauncherPacket};

pub fn restart_steamvr() -> StrResult {
    send_launcher_packet(LauncherPacket {
        message: LauncherMessages::RestartSteamvr,
    });
    Ok(())
}

pub fn invoke_application_update() -> StrResult {
    send_launcher_packet(LauncherPacket {
        message: LauncherMessages::Update,
    });
    Ok(())
}
