/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Me;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

use crate::compose::ComposePage;
use crate::edit_profile::EditProfilePage;
use crate::feed::FeedPage;
use crate::lynt_detail::LyntDetail;
use crate::my_profile_pane::MyProfilePane;
use crate::other_profile_page::OtherProfilePage;
use crate::search::SearchPage;
use crate::settings::SettingsPage;
use crate::widgets::{
    ConfirmModal, ConfirmOutcome, LyntListAction, Sidebar, SidebarItem, ToastStack,
};

enum Content {
    Placeholder,
    MyProfile(MyProfilePane),
    OtherProfile(OtherProfilePage),
    Compose(ComposePage),
    EditProfile(EditProfilePage),
    Feed(FeedPage),
    Search(SearchPage),
    Settings(SettingsPage),
}

#[derive(PartialEq)]
enum PageFocus {
    Sidebar,
    Content,
    Detail,
}

pub struct MainPage {
    sidebar: Sidebar,
    pub toasts: ToastStack,
    me: Option<Me>,
    content: Content,
    detail: Option<LyntDetail>,
    focus: PageFocus,
    confirm_modal: Option<ConfirmModal>,
    pub signed_out: bool,
    pub switch_account_requested: bool,
}

impl MainPage {
    pub fn new(me: Me) -> Self {
        Self {
            sidebar: Sidebar::new(me.user.username.clone(), me.user.handle.clone()),
            toasts: ToastStack::default(),
            me: Some(me),
            content: Content::Placeholder,
            detail: None,
            focus: PageFocus::Sidebar,
            confirm_modal: None,
            signed_out: false,
            switch_account_requested: false,
        }
    }

    pub fn tick(&mut self) {
        self.toasts.tick();

        let mut list_action: Option<LyntListAction> = None;
        let mut open_edit_requested = false;
        let mut edit_profile_closed: Option<Option<Me>> = None;

        match &mut self.content {
            Content::Compose(compose) => compose.tick(),
            Content::Feed(feed) => {
                feed.tick();
                list_action = feed.take_action();
            }
            Content::Search(search) => {
                search.tick();
                list_action = search.take_action();
            }
            Content::MyProfile(pane) => {
                pane.tick();
                list_action = pane.take_action();
                if pane.open_edit {
                    pane.open_edit = false;
                    open_edit_requested = true;
                }
            }
            Content::OtherProfile(page) => {
                page.tick();
                list_action = page.take_action();
            }
            Content::EditProfile(edit) => {
                edit.tick();
                if edit.should_close() {
                    edit_profile_closed = Some(edit.take_me());
                }
            }
            Content::Settings(_) | Content::Placeholder => {}
        }

        // These re-assign `self.content` so they happen after the match has fully released its borrow, this avoids fighting the Rust borrow checker.
        if open_edit_requested {
            if let Some(me) = self.me.take() {
                self.content = Content::EditProfile(EditProfilePage::new(me));
            }
        }
        if let Some(me_opt) = edit_profile_closed {
            if let Some(me) = me_opt {
                self.sidebar.set_display_name(me.user.username.clone());
                self.me = Some(me);
            }
            let handle = self
                .me
                .as_ref()
                .map(|m| m.user.handle.clone())
                .unwrap_or_default();
            self.content = Content::MyProfile(MyProfilePane::new(handle));
        }
        if let Some(action) = list_action {
            match action {
                LyntListAction::Open(lynt) => {
                    self.detail = Some(LyntDetail::new(lynt));
                    self.focus = PageFocus::Detail;
                }
                LyntListAction::ViewProfile(handle) => {
                    self.content = Content::OtherProfile(OtherProfilePage::new(handle));
                }
            }
        }

        let mut navigate_to = None;
        let mut navigate_to_profile = None;
        if let Some(detail) = &mut self.detail {
            detail.tick();
            navigate_to = detail.take_navigate_to();
            navigate_to_profile = detail.take_navigate_to_profile();
        }
        if let Some(lynt) = navigate_to {
            self.detail = Some(LyntDetail::new(lynt));
        } else if let Some(handle) = navigate_to_profile {
            self.detail = None;
            self.content = Content::OtherProfile(OtherProfilePage::new(handle));
            self.focus = PageFocus::Content;
        }
    }

    pub fn is_editing(&self) -> bool {
        if self.confirm_modal.is_some() {
            return false;
        }
        if let Some(detail) = &self.detail {
            if detail.is_editing() {
                return true;
            }
        }
        match &self.content {
            Content::Compose(compose) => compose.is_editing(),
            Content::EditProfile(edit) => edit.is_editing(),
            Content::Search(search) => search.is_editing(),
            _ => false,
        }
    }

    pub fn exit_editing(&mut self) {
        if self.confirm_modal.is_some() {
            self.confirm_modal = None;
            return;
        }
        if let Some(detail) = &mut self.detail {
            if detail.is_editing() {
                detail.exit_editing();
                return;
            }
        }
        if self.focus == PageFocus::Detail {
            self.detail = None;
            self.focus = PageFocus::Content;
            return;
        }
        match &mut self.content {
            Content::Compose(compose) => compose.exit_editing(),
            Content::Search(search) => search.exit_editing(),
            Content::EditProfile(edit) => {
                if edit.exit_editing() {
                    if let Some(me) = edit.take_me() {
                        self.me = Some(me);
                    }
                    self.content = Content::MyProfile(MyProfilePane::new(
                        self.me
                            .as_ref()
                            .map(|m| m.user.handle.clone())
                            .unwrap_or_default(),
                    ));
                }
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, kind: KeyEventKind, shift: bool) {
        if kind != KeyEventKind::Press {
            return;
        }

        if let Some(modal) = &mut self.confirm_modal {
            if let Some(outcome) = modal.handle_key(code) {
                self.confirm_modal = None;
                if matches!(outcome, ConfirmOutcome::Yes) {
                    crate::creds::remove(&crate::client::current_client_id());
                    crate::client::clear();
                    self.signed_out = true;
                }
            }
            return;
        }

        if shift && !self.is_editing() {
            match code {
                KeyCode::Left | KeyCode::Right => {
                    self.switch_pane(code);
                    return;
                }
                KeyCode::Up | KeyCode::Down if self.focus == PageFocus::Content => {
                    match &mut self.content {
                        Content::Feed(feed) => feed.handle_shift_vertical(code),
                        Content::Search(search) => search.handle_shift_vertical(code),
                        Content::MyProfile(pane) => pane.handle_shift_vertical(code),
                        _ => {}
                    }
                    return;
                }
                _ => {}
            }
        }
        if code == KeyCode::Tab && !self.is_editing() {
            self.focus = match self.focus {
                PageFocus::Sidebar => PageFocus::Content,
                PageFocus::Content if self.detail.is_some() => PageFocus::Detail,
                PageFocus::Content => PageFocus::Sidebar,
                PageFocus::Detail => PageFocus::Sidebar,
            };
            return;
        }

        match self.focus {
            PageFocus::Sidebar => {
                if let Some(item) = self.sidebar.handle_key(code) {
                    match item {
                        SidebarItem::Profile => {
                            if !matches!(self.content, Content::MyProfile(_)) {
                                let handle = self
                                    .me
                                    .as_ref()
                                    .map(|m| m.user.handle.clone())
                                    .unwrap_or_default();
                                self.content = Content::MyProfile(MyProfilePane::new(handle));
                            }
                            self.focus = PageFocus::Content;
                        }
                        SidebarItem::Home => {
                            if !matches!(self.content, Content::Compose(_)) {
                                self.content = Content::Compose(ComposePage::new());
                            }
                            self.focus = PageFocus::Content;
                        }
                        SidebarItem::Feed => {
                            if !matches!(self.content, Content::Feed(_)) {
                                self.content = Content::Feed(FeedPage::new());
                            }
                            self.focus = PageFocus::Content;
                        }
                        SidebarItem::Search => {
                            if !matches!(self.content, Content::Search(_)) {
                                self.content = Content::Search(SearchPage::new());
                            }
                            self.focus = PageFocus::Content;
                        }
                        SidebarItem::Settings => {
                            self.content = Content::Settings(SettingsPage::new());
                            self.focus = PageFocus::Content;
                        }
                        SidebarItem::AccountSwitcher => {
                            self.switch_account_requested = true;
                        }
                        SidebarItem::SignOut => {
                            let msg = format!("Sign out of @{}?", self.sidebar.username);
                            self.confirm_modal = Some(ConfirmModal::new(msg));
                        }
                        #[allow(unreachable_patterns)]
                        _ => {}
                    }
                }
            }
            PageFocus::Content => match &mut self.content {
                Content::Compose(compose) => compose.handle_key(code),
                Content::EditProfile(edit) => edit.handle_key(code),
                Content::Feed(feed) => feed.handle_key(code),
                Content::Search(search) => search.handle_key(code),
                Content::MyProfile(pane) => pane.handle_key(code),
                Content::OtherProfile(page) => page.handle_key(code),
                Content::Settings(_) | Content::Placeholder => {}
            },
            PageFocus::Detail => {
                if let Some(detail) = &mut self.detail {
                    detail.handle_key(code);
                }
            }
        }
    }

    fn switch_pane(&mut self, code: KeyCode) {
        match (code, &self.focus) {
            (KeyCode::Right, PageFocus::Sidebar) => self.focus = PageFocus::Content,
            (KeyCode::Right, PageFocus::Content) if self.detail.is_some() => {
                self.focus = PageFocus::Detail
            }
            (KeyCode::Left, PageFocus::Content) => self.focus = PageFocus::Sidebar,
            (KeyCode::Left, PageFocus::Detail) => self.focus = PageFocus::Content,
            _ => {}
        }
    }

    pub fn hints(&self, shift_held: bool) -> (Vec<(String, String)>, bool) {
        if self.confirm_modal.is_some() {
            return (
                vec![
                    ("←/→".into(), "Choose".into()),
                    ("↵".into(), "Confirm".into()),
                ],
                false,
            );
        }
        if shift_held {
            let pane_hints = self.pane_shift_hints();
            if !pane_hints.is_empty() {
                return (pane_hints, true);
            }
        }
        (self.normal_hints(), false)
    }

    fn normal_hints(&self) -> Vec<(String, String)> {
        let mut hints = match self.focus {
            PageFocus::Sidebar => vec![
                ("↓/J".into(), "Hover Down".into()),
                ("↑/K".into(), "Hover Up".into()),
                ("↵".into(), "Press".into()),
            ],
            PageFocus::Content => match &self.content {
                Content::Compose(compose) => compose.hints(),
                Content::EditProfile(edit) => edit.hints(),
                Content::Feed(feed) => feed.hints(),
                Content::Search(search) => search.hints(),
                Content::MyProfile(pane) => pane.hints(),
                Content::OtherProfile(page) => page.hints(),
                Content::Settings(_) | Content::Placeholder => vec![],
            },
            PageFocus::Detail => self.detail.as_ref().map(|d| d.hints()).unwrap_or_default(),
        };
        hints.push(("⇥".into(), "Next Pane".into()));
        hints
    }

    fn pane_shift_hints(&self) -> Vec<(String, String)> {
        let mut hints = Vec::new();
        if !matches!(self.focus, PageFocus::Sidebar) {
            hints.push(("←".into(), "Left Pane".into()));
        }
        let can_go_right = match self.focus {
            PageFocus::Sidebar => true,
            PageFocus::Content => self.detail.is_some(),
            PageFocus::Detail => false,
        };
        if can_go_right {
            hints.push(("→".into(), "Right Pane".into()));
        }
        hints
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let sidebar_focused = self.focus == PageFocus::Sidebar;
        let content_focused = self.focus == PageFocus::Content;
        let detail_focused = self.focus == PageFocus::Detail;

        let constraints = if self.detail.is_some() {
            vec![
                Constraint::Length(24),
                Constraint::Min(0),
                Constraint::Length(66),
            ]
        } else {
            vec![Constraint::Length(24), Constraint::Min(0)]
        };
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        self.sidebar.draw(frame, cols[0], sidebar_focused);

        let main_border = if content_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };
        let block = Block::bordered().title("Main").border_style(main_border);
        let inner = block.inner(cols[1]);
        frame.render_widget(block, cols[1]);
        match &self.content {
            Content::Placeholder => {
                frame.render_widget(
                    Paragraph::new("Select a tab on the left to get started!"),
                    inner,
                );
            }
            Content::MyProfile(pane) => {
                if let Some(me) = &self.me {
                    pane.draw(frame, inner, me, content_focused);
                }
            }
            Content::OtherProfile(page) => page.draw(frame, inner, content_focused),
            Content::Compose(compose) => compose.draw(frame, inner, content_focused),
            Content::EditProfile(edit) => edit.draw(frame, inner),
            Content::Feed(feed) => feed.draw(frame, inner, content_focused),
            Content::Search(search) => search.draw(frame, inner, content_focused),
            Content::Settings(settings) => settings.draw(frame, inner),
        }

        if let Some(detail) = &self.detail {
            let detail_border = if detail_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            let detail_block = Block::bordered().title("Post").border_style(detail_border);
            let detail_inner = detail_block.inner(cols[2]);
            frame.render_widget(detail_block, cols[2]);
            detail.draw(frame, detail_inner, detail_focused);
        }

        self.toasts.draw(frame, area);

        if let Some(modal) = &self.confirm_modal {
            modal.draw(frame, area);
        }
    }
}
