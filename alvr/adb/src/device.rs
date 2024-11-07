use crate::{
    connection_state::{self, ConnectionState},
    transport_type::{self, TransportType},
};

// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/transport.cpp;l=1409
// The serial number is printed with a "%-22s" format, meaning that it's a left-aligned space-padded string of 22 characters.
const SERIAL_NUMBER_COLUMN_LENGTH: usize = 22;

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

pub fn parse(line: &str) -> Option<Device> {
    if line.len() < SERIAL_NUMBER_COLUMN_LENGTH {
        return None;
    }
    let (left, right) = line.split_at(SERIAL_NUMBER_COLUMN_LENGTH);
    let serial = left
        .contains("(no serial number)")
        .then(|| left.trim().to_owned());
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
        connection_state::parse(left)
    } else {
        None
    };

    let mut slices = remaining.split_whitespace();
    let product = slices.next().and_then(parse_pair);
    let model = slices.next().and_then(parse_pair);
    let device = slices.next().and_then(parse_pair);
    let transport_type = slices.next().and_then(transport_type::parse);

    Some(Device {
        connection_state,
        device,
        model,
        product,
        serial,
        transport_type,
    })
}

fn parse_pair(pair: &str) -> Option<String> {
    let mut slice = pair.split(':');
    let _key = slice.next();

    slice.next().map(|value| value.to_string())
}
