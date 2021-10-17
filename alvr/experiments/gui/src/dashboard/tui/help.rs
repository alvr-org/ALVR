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
        Spans::from("To modify the settings, please close ALVR and edit settings.json."),
    ]);
    frame.render_widget(text, area);
}
