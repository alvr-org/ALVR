use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

pub struct NotificationBar {
    text: String,
    color: Color,
}

impl NotificationBar {
    pub fn new() -> Self {
        Self {
            text: "INFO: Test".into(),
            color: Color::LightBlue,
        }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {
        let bar = Paragraph::new(Text::from(Span::styled(
            self.text.clone(),
            Style::default().fg(Color::Black),
        )))
        .block(Block::default().style(Style::default().bg(self.color)));
        frame.render_widget(bar, area);
    }
}
