#[derive(Debug)]
pub struct ForwardedPort {
    pub local: u16,
    pub remote: u16,
    pub serial: String,
}

pub fn parse(line: &str) -> Option<ForwardedPort> {
    let mut slices = line.split_whitespace();
    let serial = slices.next();
    let local = parse_port(slices.next()?);
    let remote = parse_port(slices.next()?);

    if let (Some(serial), Some(local), Some(remote)) = (serial, local, remote) {
        Some(ForwardedPort {
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
