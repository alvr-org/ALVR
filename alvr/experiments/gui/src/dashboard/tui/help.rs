use tui::{backend::Backend, layout::Rect, text::Spans, widgets::Paragraph, Frame};

pub fn draw_help_panel<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let text = Paragraph::new(vec![
        Spans::from(""),
        Spans::from("Use left/right arrow keys to select the tab."),
        Spans::from("Use up/down arrow keys + 'Enter' to interact within a tab."),
        Spans::from(""),
        Spans::from("Press 'C' to enter a command."),
        Spans::from("Press 'Enter' to submit the command, or 'Esc' to cancel."),
        Spans::from(""),
        Spans::from("Press 'Ctrl+C' to exit."),
        Spans::from(""),
        Spans::from("To modify the settings, please close ALVR and edit session.json."),
    ]);
    frame.render_widget(text, area);
}
