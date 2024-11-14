// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/transport.cpp;l=1409
// The serial number is printed with a "%-22s" format, meaning that it's a left-aligned space-padded string of 22 characters.
const SERIAL_NUMBER_COLUMN_LENGTH: usize = 22;

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

pub fn parse_connection_state(value: &str) -> Option<ConnectionState> {
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

fn parse_pair(pair: &str) -> Option<String> {
    let mut slice = pair.split(':');
    let _key = slice.next();

    slice.next().map(|value| value.to_string())
}

// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/adb.h;l=95-100
#[derive(Debug)]
pub enum TransportType {
    Usb,
    Local,
    Any,
    Host,
}

pub fn parse_transport_type(pair: &str) -> Option<TransportType> {
    let mut slice = pair.split(':');
    let _key = slice.next();

    if let Ok(value) = slice.next()?.parse::<u8>() {
        match value {
            0 => Some(TransportType::Usb),
            1 => Some(TransportType::Local),
            2 => Some(TransportType::Any),
            3 => Some(TransportType::Host),
            _ => None,
        }
    } else {
        None
    }
}

// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/transport.cpp;l=1398
#[derive(Debug)]
pub struct Device {
    pub connection_state: Option<ConnectionState>,
    pub device: Option<String>,
    pub model: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub transport_type: Option<TransportType>,
}

pub fn parse_device(line: &str) -> Option<Device> {
    if line.len() < SERIAL_NUMBER_COLUMN_LENGTH {
        return None;
    }
    let (left, right) = line.split_at(SERIAL_NUMBER_COLUMN_LENGTH);
    let serial = (!left.contains("(no serial number)")).then(|| left.trim().to_owned());
    let mut remaining = right.trim();

    let connection_state = if remaining.starts_with("no permissions") {
        // Since the current user's name can be printed in the error message,
        // we are gambling that there's not a "]" in it.
        if let Some((_, right)) = remaining.split_once(']') {
            remaining = right;
            Some(ConnectionState::NoPermissions)
        } else {
            None
        }
    } else if let Some((left, right)) = remaining.split_once(' ') {
        remaining = right;
        parse_connection_state(left)
    } else {
        None
    };

    let mut slices = remaining.split_whitespace();
    let product = slices.next().and_then(parse_pair);
    let model = slices.next().and_then(parse_pair);
    let device = slices.next().and_then(parse_pair);
    let transport_type = slices.next().and_then(parse_transport_type);

    Some(Device {
        connection_state,
        device,
        model,
        product,
        serial,
        transport_type,
    })
}

#[derive(Debug)]
pub struct ForwardedPorts {
    pub local: u16,
    pub remote: u16,
    pub serial: String,
}

pub fn parse_forwarded_ports(line: &str) -> Option<ForwardedPorts> {
    let mut slices = line.split_whitespace();
    let serial = slices.next();
    let local = parse_port(slices.next()?);
    let remote = parse_port(slices.next()?);

    if let (Some(serial), Some(local), Some(remote)) = (serial, local, remote) {
        Some(ForwardedPorts {
            local,
            remote,
            serial: serial.to_owned(),
        })
    } else {
        None
    }
}

fn parse_port(value: &str) -> Option<u16> {
    let mut slices = value.split(':');
    let _protocol = slices.next();
    let maybe_port = slices.next();

    maybe_port.and_then(|p| p.parse::<u16>().ok())
}
