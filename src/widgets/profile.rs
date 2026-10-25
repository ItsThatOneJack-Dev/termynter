/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Me;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::widgets::Button;

const ICON_VERIFIED: &str = "\u{f0791}";
const ICON_ADMIN: &str = "\u{f1322}";
const ICON_IQ: &str = "\u{ee9c}";

/// `lyntr-rs`'s `client.user()` return type isn't namable due to apparently not being exported anywhere, so this is a copy of only the fields we need, built with type inference.
pub struct OtherProfile {
    pub username: String,
    pub handle: String,
    pub bio: String,
    pub iq: i64,
    pub verified: bool,
    pub contributor: bool,
    pub follower_count: i64,
    pub following_count: i64,
    pub created_at: String,
}

/// A borrowed view over whichever profile shape is being shown, `Me` knows `is_admin` and `lynt_coins` in this setup, while `OtherProfile` instead knows follower and following counts.
pub enum ProfileRef<'a> {
    Mine(&'a Me),
    Other(&'a OtherProfile),
}

impl<'a> ProfileRef<'a> {
    fn username(&self) -> &str {
        match self {
            Self::Mine(me) => &me.user.username,
            Self::Other(p) => &p.username,
        }
    }
    fn handle(&self) -> &str {
        match self {
            Self::Mine(me) => &me.user.handle,
            Self::Other(p) => &p.handle,
        }
    }
    fn bio(&self) -> &str {
        match self {
            Self::Mine(me) => &me.user.bio,
            Self::Other(p) => &p.bio,
        }
    }
    fn iq(&self) -> i64 {
        match self {
            Self::Mine(me) => me.user.iq,
            Self::Other(p) => p.iq,
        }
    }
    fn verified(&self) -> bool {
        match self {
            Self::Mine(me) => me.user.verified,
            Self::Other(p) => p.verified,
        }
    }
    fn created_at(&self) -> &str {
        match self {
            Self::Mine(me) => &me.user.created_at,
            Self::Other(p) => &p.created_at,
        }
    }
}

/// Draws a profile, `edit_focused` highlights the "Edit profile" button, the button doesn't appear when `subject` is not `Mine`.
pub fn draw(frame: &mut Frame, area: Rect, subject: ProfileRef, edit_focused: bool) {
    let has_edit_button = matches!(subject, ProfileRef::Mine(_));
    let has_follow_stats = matches!(subject, ProfileRef::Other(_));

    let mut constraints = vec![
        Constraint::Length(1), // Display name.
        Constraint::Length(1), // Handle.
        Constraint::Length(1), // Badges / IQ.
        Constraint::Length(1), // Spacer.
        Constraint::Length(1), // Join date.
    ];
    if has_follow_stats {
        constraints.push(Constraint::Length(1)); // Spacer.
        constraints.push(Constraint::Length(1)); // Follower count.
        constraints.push(Constraint::Length(1)); // Following count.
    }
    constraints.push(Constraint::Length(1)); // Spacer.
    constraints.push(Constraint::Min(3)); // Bio pane.
    if has_edit_button {
        constraints.push(Constraint::Length(1)); // Spacer.
        constraints.push(Constraint::Length(3)); // Edit button.
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    let mut i = 0usize;

    frame.render_widget(
        Paragraph::new(subject.username()).style(Style::default().add_modifier(Modifier::BOLD)),
        rows[i],
    );
    i += 1;
    frame.render_widget(
        Paragraph::new(format!("@{}", subject.handle()))
            .style(Style::default().fg(Color::DarkGray)),
        rows[i],
    );
    i += 1;

    let mut badges: Vec<Span> = Vec::new();
    if let ProfileRef::Mine(me) = &subject {
        if me.user.is_admin {
            badges.push(Span::styled(ICON_ADMIN, Style::default().fg(Color::Red)));
            badges.push(Span::raw(" "));
        }
    }
    if subject.verified() {
        badges.push(Span::styled(
            ICON_VERIFIED,
            Style::default().fg(Color::Cyan),
        ));
        badges.push(Span::raw(" "));
    }
    if let ProfileRef::Other(p) = &subject {
        if p.contributor {
            badges.push(Span::styled("★", Style::default().fg(Color::Yellow)));
            badges.push(Span::raw(" "));
        }
    }
    badges.push(Span::styled(ICON_IQ, Style::default().fg(Color::Yellow)));
    badges.push(Span::raw(subject.iq().to_string()));
    frame.render_widget(Paragraph::new(Line::from(badges)), rows[i]);
    i += 2; // Skip the spacer row after badges.

    frame.render_widget(
        Paragraph::new(format!(
            "Joined Lyntr on {}",
            format_joined_date(subject.created_at())
        )),
        rows[i],
    );
    i += 1;

    if let ProfileRef::Other(p) = &subject {
        i += 1; // Spacer.
        let follower_label = if p.follower_count == 1 {
            "Follower"
        } else {
            "Followers"
        };
        frame.render_widget(
            Paragraph::new(format!("{} {}", p.follower_count, follower_label)),
            rows[i],
        );
        i += 1;
        frame.render_widget(
            Paragraph::new(format!("{} Following", p.following_count)),
            rows[i],
        );
        i += 1;
    }

    i += 1; // Spacer before bio.
    let bio_block = Block::bordered().title("Bio");
    let bio_inner = bio_block.inner(rows[i]);
    frame.render_widget(bio_block, rows[i]);
    let bio_text = if subject.bio().trim().is_empty() {
        "Nothing here yet..."
    } else {
        subject.bio()
    };
    frame.render_widget(
        Paragraph::new(bio_text).wrap(Wrap { trim: true }),
        bio_inner,
    );
    i += 1;

    if has_edit_button {
        i += 1; // Spacer.
        Button::new("Edit Profile").draw(frame, rows[i], edit_focused);
    }
}

fn format_joined_date(created_at: &str) -> String {
    let date_part = created_at.split('T').next().unwrap_or(created_at);
    match date_part.split('-').collect::<Vec<_>>().as_slice() {
        [year, month, day] => format!("{day}/{month}/{year}"),
        _ => created_at.to_string(),
    }
}
