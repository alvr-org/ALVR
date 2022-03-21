use super::reset;

pub struct Control {
    default: usize,
    entries: Vec<String>,
    selection: usize,
    reset_control: reset::Control,
}
