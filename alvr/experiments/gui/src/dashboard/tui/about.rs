use alvr_session::SessionDesc;
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_about_panel<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let text = Paragraph::new(vec![
        Spans::from(""),
        Spans::from(Span::styled(
            "ALVR server vX.X.X",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Spans::from("GitHub: github.com/alvr-org/alvr"),
        Spans::from("Discord: discord.gg/alvr"),
    ])
    .alignment(Alignment::Center);

    frame.render_widget(text, area);
}
