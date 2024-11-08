// https://cs.android.com/android/platform/superproject/main/+/7dbe542b9a93fb3cee6c528e16e2d02a26da7cc0:packages/modules/adb/adb.h;l=95-100
#[derive(Debug)]
pub enum TransportType {
    Usb,
    Local,
    Any,
    Host,
}

pub fn parse(pair: &str) -> Option<TransportType> {
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
