// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/adb.h;l=104-122
#[derive(Debug)]
pub enum ConnectionState {
    Authorizing,
    Bootloader,
    Connecting,
    Detached,
    Device,
    Host,
    NoPermissions, // https://cs.android.com/android/platform/superproject/main/+/main:system/core/diagnose_usb/diagnose_usb.cpp;l=83-90
    Offline,
    Recovery,
    Rescue,
    Sideload,
    Unauthorized,
}

pub fn parse(value: &str) -> Option<ConnectionState> {
    match value {
        "authorizing" => Some(ConnectionState::Authorizing),
        "bootloader" => Some(ConnectionState::Bootloader),
        "connecting" => Some(ConnectionState::Connecting),
        "detached" => Some(ConnectionState::Detached),
        "device" => Some(ConnectionState::Device),
        "host" => Some(ConnectionState::Host),
        "offline" => Some(ConnectionState::Offline),
        "recovery" => Some(ConnectionState::Recovery),
        "rescue" => Some(ConnectionState::Rescue),
        "sideload" => Some(ConnectionState::Sideload),
        "unauthorized" => Some(ConnectionState::Unauthorized),
        _ => None,
    }
}
