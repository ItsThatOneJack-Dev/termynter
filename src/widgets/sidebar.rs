/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarItem {
    Home,
    Feed,
    Search,
    Profile,
    Settings,
    AccountSwitcher,
    SignOut,
}

impl SidebarItem {
    pub fn label(self) -> &'static str {
        match self {
            SidebarItem::Home => "Home",
            SidebarItem::Feed => "Feed",
            SidebarItem::Search => "Search",
            SidebarItem::Profile => "Profile",
            SidebarItem::Settings => "Settings",
            SidebarItem::AccountSwitcher => "Switch Account",
            SidebarItem::SignOut => "Sign Out",
        }
    }
}

// ITEMS is the selectable sequence, used for the cycle via Up/Down controls, while ROWS is what is drawn (None is a spacer row).
const ITEMS: &[SidebarItem] = &[
    SidebarItem::Home,
    SidebarItem::Feed,
    SidebarItem::Search,
    SidebarItem::Profile,
    SidebarItem::Settings,
    SidebarItem::AccountSwitcher,
    SidebarItem::SignOut,
];

const ROWS: &[Option<SidebarItem>] = &[
    Some(SidebarItem::Home),
    Some(SidebarItem::Feed),
    Some(SidebarItem::Search),
    None,
    Some(SidebarItem::Profile),
    None,
    Some(SidebarItem::Settings),
    Some(SidebarItem::AccountSwitcher),
    Some(SidebarItem::SignOut),
];

pub struct Sidebar {
    selected: usize, // The index into ITEMS.
    pub display_name: String,
    pub username: String,
}

impl Sidebar {
    pub fn new(display_name: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            selected: 0,
            display_name: display_name.into(),
            username: username.into(),
        }
    }

    pub fn set_display_name(&mut self, display_name: impl Into<String>) {
        self.display_name = display_name.into();
    }

    pub fn selected(&self) -> SidebarItem {
        ITEMS[self.selected]
    }

    pub fn handle_key(&mut self, code: KeyCode) -> Option<SidebarItem> {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.selected = (self.selected + 1).min(ITEMS.len() - 1);
                None
            }
            KeyCode::Enter => Some(self.selected()),
            _ => None,
        }
    }

    /// When selected, the hovered item inside always shows yellow regardless as you can't focus a button.
    pub fn draw(&self, frame: &mut Frame, area: Rect, selected: bool) {
        let border_style = if selected {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };
        let block = Block::bordered().title("Menu").border_style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(4)])
            .split(inner);

        let list_items: Vec<ListItem> = ROWS
            .iter()
            .map(|row| ListItem::new(row.map(SidebarItem::label).unwrap_or("")))
            .collect();

        let mut state = ListState::default();
        let selected_row = ROWS
            .iter()
            .enumerate()
            .filter_map(|(i, row)| row.map(|_| i))
            .nth(self.selected);
        state.select(selected_row);

        let highlight_style = if selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        };
        let list = List::new(list_items).highlight_style(highlight_style);
        frame.render_stateful_widget(list, sections[0], &mut state);

        let user_block = Block::bordered();
        let user_inner = user_block.inner(sections[1]);
        frame.render_widget(user_block, sections[1]);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(self.display_name.as_str()),
                Line::from(format!("@{}", self.username)),
            ]),
            user_inner,
        );
    }
}
