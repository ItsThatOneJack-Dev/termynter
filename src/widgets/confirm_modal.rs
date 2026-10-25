/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

pub enum ConfirmOutcome {
    Yes,
    No,
}

pub struct ConfirmModal {
    message: String,
    yes_selected: bool,
}

impl ConfirmModal {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            yes_selected: false,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> Option<ConfirmOutcome> {
        match code {
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::Char('h')
            | KeyCode::Char('H')
            | KeyCode::Char('l')
            | KeyCode::Char('L') => {
                self.yes_selected = !self.yes_selected;
                None
            }
            KeyCode::Enter => Some(if self.yes_selected {
                ConfirmOutcome::Yes
            } else {
                ConfirmOutcome::No
            }),
            _ => None,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let width = (self.message.len() as u16 + 8).clamp(24, area.width.saturating_sub(4));
        let rect = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(5)) / 2,
            width,
            height: 5,
        };
        frame.render_widget(Clear, rect);
        let block = Block::bordered().border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        frame.render_widget(
            Paragraph::new(self.message.as_str()).alignment(Alignment::Center),
            rows[0],
        );

        let yes_style = if self.yes_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default()
        };
        let no_style = if !self.yes_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default()
        };
        let line = Line::from(vec![
            Span::styled(" Yes ", yes_style),
            Span::raw("   "),
            Span::styled(" No ", no_style),
        ]);
        frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), rows[1]);
    }
}
