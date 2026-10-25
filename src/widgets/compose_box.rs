/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph, Wrap};

pub struct ComposeBox {
    value: String,
    cursor: usize,
    pub locked: bool,
}

impl ComposeBox {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            locked: false,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.chars().count();
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.locked {
            return false;
        }
        match code {
            KeyCode::Char(c) => {
                let idx = self.byte_index(self.cursor);
                self.value.insert(idx, c);
                self.cursor += 1;
                true
            }
            KeyCode::Enter => {
                let idx = self.byte_index(self.cursor);
                self.value.insert(idx, '\n');
                self.cursor += 1;
                true
            }
            KeyCode::Backspace if self.cursor > 0 => {
                let idx = self.byte_index(self.cursor - 1);
                self.value.remove(idx);
                self.cursor -= 1;
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                false
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.value.chars().count());
                false
            }
            KeyCode::Up => {
                self.move_vertical(-1);
                false
            }
            KeyCode::Down => {
                self.move_vertical(1);
                false
            }
            _ => false,
        }
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    fn move_vertical(&mut self, delta: i32) {
        let chars: Vec<char> = self.value.chars().collect();
        let mut line_starts = vec![0usize];
        for (i, &c) in chars.iter().enumerate() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        let mut current_line = 0usize;
        let mut line_start = 0usize;
        for (li, &start) in line_starts.iter().enumerate() {
            if start <= self.cursor {
                current_line = li;
                line_start = start;
            }
        }
        let col = self.cursor - line_start;

        let target_line = current_line as i32 + delta;
        if target_line < 0 || target_line as usize >= line_starts.len() {
            return;
        }
        let target_line = target_line as usize;
        let target_start = line_starts[target_line];
        let target_end = line_starts
            .get(target_line + 1)
            .map(|&s| s - 1)
            .unwrap_or(chars.len());
        self.cursor = target_start + col.min(target_end - target_start);
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, hovered: bool, editing: bool, title: &str) {
        let mut display = self.value.clone();
        if editing && !self.locked {
            let idx = self.byte_index(self.cursor);
            display.insert(idx, '│');
        }
        let text_style = if self.locked {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let border_style = if editing {
            Style::default().fg(Color::Green)
        } else if hovered {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(display)
                .style(text_style)
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .title(title.to_string())
                        .border_style(border_style),
                ),
            area,
        );
    }
}
