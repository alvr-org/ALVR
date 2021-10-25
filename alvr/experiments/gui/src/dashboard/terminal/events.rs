use std::{cmp, collections::VecDeque};

use alvr_common::ServerEvent;
use tui::{backend::Backend, layout::Rect, text::Spans, widgets::Paragraph, Frame};

const MAX_EVENTS: usize = 50;

pub struct EventsPanel {
    event_buffer: VecDeque<ServerEvent>,
}

impl EventsPanel {
    pub fn new() -> Self {
        Self {
            event_buffer: VecDeque::new(),
        }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let lines = (0..area.height).into_iter().map(|line_index| {
            let index = self.event_buffer.len() as isize - (area.height - line_index) as isize;
            let text = if index >= 0 {
                format!("{:?}", self.event_buffer[index as usize])
            } else {
                String::new()
            };

            Spans::from(text) // todo: format with colors
        });

        let lines = Paragraph::new(lines.collect::<Vec<_>>());
        frame.render_widget(lines, area);
    }

    pub fn push_events(&mut self, events: Vec<ServerEvent>) {
        for e in events {
            self.event_buffer.push_back(e);
        }
        if self.event_buffer.len() >= MAX_EVENTS {
            self.event_buffer
                .drain(..self.event_buffer.len() - MAX_EVENTS);
        }
    }
}
