use iced::{Element, Text};

#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    ClientButtonClick(String),
}

pub struct ConnectionPanel {}

impl Default for ConnectionPanel {
    fn default() -> Self {
        Self {}
    }
}

impl ConnectionPanel {
    pub fn update(
        &mut self,
        event: ConnectionEvent,
        request_handler: &mut dyn FnMut(String) -> String,
    ) {
        match event {
            ConnectionEvent::ClientButtonClick(hostname) => (),
        }
    }

    pub fn view(&mut self) -> Element<ConnectionEvent> {
        Text::new("test").into()
    }
}
