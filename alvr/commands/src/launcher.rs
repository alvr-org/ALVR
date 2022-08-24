use alvr_common::{prelude::*, send_control_packet, ControlMessages, ControlPacket};

pub fn restart_steamvr() -> StrResult {
    send_control_packet(ControlPacket {
        message: ControlMessages::RestartSteamvr,
    });
    Ok(())
}

pub fn invoke_application_update() -> StrResult {
    send_control_packet(ControlPacket {
        message: ControlMessages::Update,
    });
    Ok(())
}
