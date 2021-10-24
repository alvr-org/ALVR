use std::{cmp, collections::VecDeque};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    text::Spans,
    widgets::Paragraph,
    Frame,
};

pub struct ReplPanel {
    history_buffer: VecDeque<String>,
    last_command: String,
    temp_command: String,
    cursor_position: usize,
    history_position: usize,
}

impl ReplPanel {
    pub fn new() -> Self {
        Self {
            history_buffer: VecDeque::new(),
            last_command: "".into(),
            temp_command: "".into(),
            cursor_position: 0,
            history_position: 0,
        }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let history_lines = (0..chunks[0].height).into_iter().map(|line_index| {
            let index = (self.history_buffer.len() - self.history_position) as isize
                - (area.height - line_index) as isize
                + 1;
            let text = if index >= 0 {
                self.history_buffer[index as usize].clone()
            } else {
                String::new()
            };

            Spans::from(text) // todo: format with colors
        });
        let history_lines = Paragraph::new(history_lines.collect::<Vec<_>>());
        frame.render_widget(history_lines, chunks[0]);

        let prompt_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(2), Constraint::Min(0)])
            .split(chunks[1]);

        let prompt_chevron = Paragraph::new(">");
        frame.render_widget(prompt_chevron, prompt_chunks[0]);

        let prompt_command = Paragraph::new(vec![Spans::from(self.temp_command.clone())]).scroll((
            0,
            cmp::max(
                0,
                self.cursor_position as i16 - prompt_chunks[1].width as i16 + 1,
            ) as u16,
        ));
        frame.render_widget(prompt_command, prompt_chunks[1]);

        frame.set_cursor(
            prompt_chunks[1].x + self.cursor_position as u16,
            prompt_chunks[1].y,
        );
    }

    pub fn react_to_key(&mut self, key: Key, request_handler: &mut impl FnMut(String) -> String) {
        match key {
            Key::PageDown if self.history_position > 0 => self.history_position -= 1,
            Key::PageUp => {
                self.history_position =
                    cmp::min(self.history_position + 1, self.history_buffer.len() - 1)
            }
            Key::Left if self.cursor_position > 0 => self.cursor_position -= 1,
            Key::Right => {
                self.cursor_position = cmp::min(self.cursor_position + 1, self.temp_command.len())
            }
            Key::Up => {
                self.temp_command = self.last_command.clone();
                self.cursor_position = self.temp_command.len();
            }
            Key::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.temp_command.remove(self.cursor_position);
                }
            }
            Key::Delete => {
                if self.cursor_position < self.temp_command.len() {
                    self.temp_command.remove(self.cursor_position);
                }
            }
            Key::Char('\n') => {
                self.history_buffer
                    .push_back(format!("> {}", self.temp_command.clone()));
                let response = request_handler(self.temp_command.clone());
                if !response.is_empty() {
                    for line in response.split('\n') {
                        self.history_buffer.push_back(line.into());
                    }
                }

                self.last_command = self.temp_command.clone();
                self.temp_command.clear();
                self.cursor_position = 0;
                self.history_position = 0;
            }
            Key::Char(char) => {
                self.temp_command.insert(self.cursor_position, char);
                self.cursor_position += 1;
            }
            _ => (),
        }
    }
}
