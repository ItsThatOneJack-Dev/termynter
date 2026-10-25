/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Lynt;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::time_format::relative_time;

pub struct LyntView<'a> {
    lynt: &'a Lynt,
    liked: bool,
    busy: bool,
    hovered: bool,
    action_focus: Option<usize>,
}

impl<'a> LyntView<'a> {
    pub fn new(lynt: &'a Lynt) -> Self {
        Self {
            lynt,
            liked: false,
            busy: false,
            hovered: false,
            action_focus: None,
        }
    }

    pub fn liked(mut self, liked: bool) -> Self {
        self.liked = liked;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Places a yellow border around the whole block, used when this view is the hovered entry in a list, e.g. a comment.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    /// Highlights one action in the row (0 = Like, 1 = Comment) yellow.
    pub fn action_focus(mut self, action_focus: Option<usize>) -> Self {
        self.action_focus = action_focus;
        self
    }

    fn body_text(&self) -> String {
        let mut content = self.lynt.content.clone();
        if self.lynt.edited_at.is_some() {
            content.push_str("\n(edited)");
        }
        content
    }

    pub fn height(&self, width: u16, max_content_lines: u16) -> u16 {
        let content_width = if self.hovered {
            width.saturating_sub(2)
        } else {
            width
        };
        let content_lines = wrapped_line_count(&self.body_text(), content_width);
        let base = 4 + content_lines.max(1).min(max_content_lines);
        if self.hovered { base + 2 } else { base }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let inner = if self.hovered {
            let block = Block::bordered().border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(area);
            frame.render_widget(block, area);
            inner
        } else {
            area
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(Paragraph::new(header_line(self.lynt)), rows[0]);
        frame.render_widget(
            Paragraph::new(self.body_text()).wrap(Wrap { trim: true }),
            rows[2],
        );
        frame.render_widget(Paragraph::new(self.actions_line()), rows[4]);
    }

    fn actions_line(&self) -> Line<'static> {
        let like_label = if self.busy {
            "Liking…".to_string()
        } else if self.liked {
            format!("Unlike ({})", self.lynt.like_count)
        } else {
            format!("Like ({})", self.lynt.like_count)
        };
        let like_style = if self.action_focus == Some(0) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let comment_label = format!("Comment ({})", self.lynt.comment_count);
        let comment_style = if self.action_focus == Some(1) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        Line::from(vec![
            Span::raw("["),
            Span::styled(like_label, like_style),
            Span::raw("] ["),
            Span::styled(comment_label, comment_style),
            Span::raw("]"),
        ])
    }
}

fn header_line(lynt: &Lynt) -> String {
    format!(
        "{} (@{}) · {}",
        lynt.username,
        lynt.handle,
        relative_time(&lynt.created_at)
    )
}

fn wrapped_line_count(text: &str, width: u16) -> u16 {
    let width = width.max(1) as usize;
    let mut count = 0u16;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            count += 1;
            continue;
        }
        let mut current_width = 0usize;
        let mut rows_for_this_line = 1u16;
        for word in line.split_whitespace() {
            let word_len = word.chars().count();
            if current_width == 0 {
                current_width = word_len;
            } else if current_width + 1 + word_len <= width {
                current_width += 1 + word_len;
            } else {
                rows_for_this_line += 1;
                current_width = word_len;
            }
        }
        count += rows_for_this_line;
    }
    count.max(1)
}
