/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, Paragraph};
use std::time::{Duration, Instant};

const HEIGHT: u16 = 3;
const GAP: u16 = 1;
const MARGIN_RIGHT: u16 = 2;
const MARGIN_BOTTOM: u16 = 1;
const DEFAULT_TTL: Duration = Duration::from_secs(3);

/// Describes a toast to show. Build with `Toast::new(...)`.
pub struct Toast {
    pub message: String,
    pub ttl: Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ttl: DEFAULT_TTL,
        }
    }
}

struct Entry {
    message: String,
    created: Instant,
    ttl: Duration,
}

#[derive(Default)]
pub struct ToastStack {
    entries: Vec<Entry>,
}

impl ToastStack {
    pub fn push(&mut self, toast: Toast) {
        self.entries.push(Entry {
            message: toast.message,
            created: Instant::now(),
            ttl: toast.ttl,
        });
    }

    /// Drops expired toasts. Call once per loop tick.
    pub fn tick(&mut self) {
        self.entries.retain(|e| e.created.elapsed() < e.ttl);
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let mut y = area.height.saturating_sub(MARGIN_BOTTOM);
        // Newest first (bottom-most), oldest stacked upward above it.
        for entry in self.entries.iter().rev() {
            if y < HEIGHT {
                break;
            }
            let width = (entry.message.len() as u16 + 4).min(area.width).max(10);
            let rect = Rect {
                x: area.width.saturating_sub(width + MARGIN_RIGHT),
                y: y.saturating_sub(HEIGHT),
                width,
                height: HEIGHT,
            };

            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(entry.message.as_str())
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::bordered()),
                rect,
            );

            y = rect.y.saturating_sub(GAP);
        }
    }
}
