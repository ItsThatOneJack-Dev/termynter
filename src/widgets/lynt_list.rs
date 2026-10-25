/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Lynt;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use super::time_format::relative_time;

pub enum LyntListAction {
    Open(Lynt),
    ViewProfile(String),
}

pub struct LyntList {
    lynts: Vec<Lynt>,
    selected: usize,
}

impl LyntList {
    pub fn new(lynts: Vec<Lynt>) -> Self {
        Self { lynts, selected: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.lynts.is_empty()
    }

    /// UpArrow/J, DownArrow/K move the hover up and down. Enter opens the hovered post. `p` views the author's profile.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<LyntListAction> {
        if self.lynts.is_empty() {
            return None;
        }
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.selected = (self.selected + 1).min(self.lynts.len() - 1);
                None
            }
            KeyCode::Enter => self
                .lynts
                .get(self.selected)
                .cloned()
                .map(LyntListAction::Open),
            KeyCode::Char('p') | KeyCode::Char('P') => self
                .lynts
                .get(self.selected)
                .map(|l| LyntListAction::ViewProfile(l.handle.clone())),
            _ => None,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, hovered: bool) {
        if self.lynts.is_empty() {
            frame.render_widget(
                Paragraph::new("Nothing here yet.").style(Style::default().fg(Color::DarkGray)),
                area,
            );
            return;
        }
        let items: Vec<ListItem> = self.lynts.iter().map(summary_item).collect();
        let mut state = ListState::default();
        state.select(Some(self.selected));
        let highlight_style = if hovered {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        };
        let list = List::new(items).highlight_style(highlight_style);
        frame.render_stateful_widget(list, area, &mut state);
    }

    pub fn replace(&mut self, mut new_lynts: Vec<Lynt>) {
        if let Some(current) = self.lynts.get(self.selected) {
            let current_id = current.id.clone();
            if let Some(pos) = new_lynts.iter().position(|l| l.id == current_id) {
                self.lynts = new_lynts;
                self.selected = pos;
            } else {
                let current = self.lynts.remove(self.selected);
                new_lynts.insert(0, current);
                self.lynts = new_lynts;
                self.selected = 0;
            }
        } else {
            self.lynts = new_lynts;
            self.selected = 0;
        }
    }
}

fn summary_item(lynt: &Lynt) -> ListItem<'static> {
    let header = Line::from(vec![
        Span::styled(
            lynt.username.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " (@{}) · {}",
            lynt.handle,
            relative_time(&lynt.created_at)
        )),
    ]);
    const PREVIEW_CHARS: usize = 80;
    let mut lines = lynt.content.lines();
    let first_line = lines.next().unwrap_or("");
    let has_more_lines = lines.next().is_some();
    let truncated = first_line.chars().count() > PREVIEW_CHARS;
    let preview: String = first_line.chars().take(PREVIEW_CHARS).collect();
    let third_line = if has_more_lines || truncated {
        Line::from(Span::styled("[...]", Style::default().fg(Color::DarkGray)))
    } else {
        Line::from("")
    };
    ListItem::new(vec![header, Line::from(preview), third_line])
}
