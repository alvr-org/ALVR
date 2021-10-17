use crate::dashboard::DashboardEvent;
use std::cmp;
use termion::event::Key;
use tui::{backend::Backend, layout::Rect, text::Text, widgets::Paragraph, Frame};

pub struct CommandBar {
    text_buffer: String,
    cursor_position: usize,
    last_command: String,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            text_buffer: "".into(),
            cursor_position: 0,
            last_command: "".into(),
        }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect, active: bool) {
        if !active {
            let prompt = Paragraph::new("Press 'c' to enter a command");
            frame.render_widget(prompt, area);
        } else {
            let prompt = Paragraph::new(Text::from(format!("> {}", self.text_buffer)));
            frame.render_widget(prompt, area);
            frame.set_cursor(area.x + 2 + self.cursor_position as u16, area.y);
        }
    }

    pub fn react_to_key(
        &mut self,
        key: Key,
        event_handler: &mut impl FnMut(DashboardEvent),
        active: &mut bool,
    ) {
        match key {
            Key::Left if self.cursor_position > 0 => self.cursor_position -= 1,
            Key::Right => {
                self.cursor_position = cmp::min(self.cursor_position + 1, self.text_buffer.len())
            }
            Key::Up => {
                self.text_buffer = self.last_command.clone();
                self.cursor_position = self.text_buffer.len();
            }
            Key::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.text_buffer.remove(self.cursor_position);
                }
            }
            Key::Delete => {
                if self.cursor_position < self.text_buffer.len() {
                    self.text_buffer.remove(self.cursor_position);
                }
            }
            Key::Char('\n') => {
                event_handler(DashboardEvent::Command(self.text_buffer.clone()));
                self.last_command = self.text_buffer.clone();
                self.text_buffer.clear();
                self.cursor_position = 0;
                *active = false;
            }
            Key::Esc => *active = false,
            Key::Char(char) => {
                self.text_buffer.insert(self.cursor_position, char);
                self.cursor_position += 1;
            }
            _ => (),
        }
    }
}
