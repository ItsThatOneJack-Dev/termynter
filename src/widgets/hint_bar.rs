/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Draws the hints bar at the bottom of the screen, e.g.: "[key] label ┃ [key] label ┃ ..."
/// `shift_prefix` renders each entry as "SHIFT+[key] label" instead.
pub fn draw(frame: &mut Frame, area: Rect, hints: &[(String, String)], shift_prefix: bool) {
    let mut spans = Vec::new();
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ┃ ", Style::default().fg(Color::DarkGray)));
        }
        if shift_prefix {
            spans.push(Span::raw("SHIFT+"));
        }
        spans.push(Span::raw(format!("[{key}] {label}")));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
