/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

/// A single-line text field, not validated internally, instead takes a call to `set_validation` after reading `.value()` whenever `handle_key` reports a value change.
pub struct TextInput {
    value: String,
    cursor: usize,
    pub locked: bool,
    validation: Option<String>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            locked: false,
            validation: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn is_valid(&self) -> bool {
        self.validation.is_none()
    }

    pub fn validation_message(&self) -> Option<&str> {
        self.validation.as_deref()
    }

    pub fn set_validation(&mut self, message: Option<String>) {
        self.validation = message;
    }

    /// Returns true if the value changed (so the caller knows to re-validate).
    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.locked {
            return false;
        }
        match code {
            KeyCode::Char(c) => {
                self.value.insert(self.cursor, c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.value.remove(self.cursor);
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                false
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.value.len());
                false
            }
            _ => false,
        }
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, title: &str, hovered: bool, editing: bool) {
        let mut display = self.value.clone();
        if !self.locked {
            display.insert(self.cursor, '│');
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
            Paragraph::new(display).style(text_style).block(
                Block::bordered()
                    .title(title.to_string())
                    .border_style(border_style),
            ),
            area,
        );
    }
}
