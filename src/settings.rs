/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

pub struct SettingsPage;

impl SettingsPage {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new("Settings coming soon.").style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }
}
