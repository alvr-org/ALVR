use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::Paragraph,
    Frame,
};

pub fn draw_about_panel<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let text = Paragraph::new(vec![
        Spans::from(""),
        Spans::from(Span::styled(
            format!("ALVR server v{}", alvr_common::ALVR_VERSION.to_string()),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Spans::from("GitHub: github.com/alvr-org/alvr"),
        Spans::from("Discord: discord.gg/alvr"),
    ])
    .alignment(Alignment::Center);

    frame.render_widget(text, area);
}
