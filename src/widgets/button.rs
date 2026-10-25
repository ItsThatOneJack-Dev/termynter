/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Button {
    pub label: String,
    pub enabled: bool,
    pub busy: bool,
    spinner_frame: usize,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            busy: false,
            spinner_frame: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.busy {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER.len();
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, hovered: bool) {
        let label = if self.busy {
            format!("{} {}", SPINNER[self.spinner_frame], self.label)
        } else {
            self.label.clone()
        };
        let text_style = if self.busy || !self.enabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let border_style = if hovered {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(text_style)
                .block(Block::bordered().border_style(border_style)),
            area,
        );
    }
}
