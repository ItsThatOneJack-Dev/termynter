/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Lynt;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::client;
use crate::widgets::{Button, ComposeBox, LyntView};

struct CommentEntry {
    lynt: Lynt,
    liked: bool,
    busy: bool,
}

enum CommentsState {
    Loading,
    Loaded(Vec<CommentEntry>),
    Failed(String),
}

fn spawn_comments_fetch(lynt: Lynt) -> Receiver<Result<Vec<Lynt>, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { lynt.comments().await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

fn spawn_like_toggle(lynt: Lynt, like: bool) -> Receiver<Result<bool, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            if like {
                lynt.like().await
            } else {
                lynt.unlike().await
            }
        });
        let _ = tx.send(result.map(|_| like).map_err(|e| e.to_string()));
    });
    rx
}

fn spawn_comment_like_toggle(
    lynt: Lynt,
    like: bool,
    id: String,
) -> Receiver<(String, Result<bool, String>)> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            if like {
                lynt.like().await
            } else {
                lynt.unlike().await
            }
        });
        let _ = tx.send((id, result.map(|_| like).map_err(|e| e.to_string())));
    });
    rx
}

fn spawn_reply(lynt: Lynt, content: String) -> Receiver<Result<Lynt, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { lynt.reply(&content).await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

fn spawn_edit(mut lynt: Lynt, content: String) -> Receiver<Result<Lynt, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { lynt.edit(&content).await.map(|_| lynt) });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

const COMPOSER_HEIGHT: u16 = 6;
const POST_BUTTON_HEIGHT: u16 = 3;
const MAX_COMMENT_LINES: u16 = 8;
const MAX_COMMENT_CHARS: usize = 280;

struct EditPostState {
    box_: ComposeBox,
    editing_box: bool,
    hovered: bool,
    save_btn: Button,
    busy: bool,
    error: Option<String>,
    job: Option<Receiver<Result<Lynt, String>>>,
}

/// The right-hand detail panel, uses a numerical indexing of hover targets (`hovered`):
/// 0 = top post, 1 = composer, 2 = post button, 3+ = comments.
/// On a lynt row (0 to 3+), `action_cursor` (0/1) picks Like vs Comment, left/right move it and enter activates it.
/// l and p are direct shortcuts regardless. e on your own top post opens the edit menu for it.
pub struct LyntDetail {
    lynt: Lynt,
    liked: bool,
    like_busy: bool,
    like_error: Option<String>,
    like_job: Option<Receiver<Result<bool, String>>>,
    comments: CommentsState,
    comments_job: Option<Receiver<Result<Vec<Lynt>, String>>>,
    comment_like_job: Option<Receiver<(String, Result<bool, String>)>>,
    composer: ComposeBox,
    editing_composer: bool,
    post_button: Button,
    post_busy: bool,
    post_error: Option<String>,
    post_job: Option<Receiver<Result<Lynt, String>>>,
    hovered: usize,
    action_cursor: usize,
    scroll_offset: usize,
    visible_end: Cell<usize>,
    navigate_to: Option<Lynt>,
    navigate_to_profile: Option<String>,
    editing_post: Option<EditPostState>,
}

impl LyntDetail {
    pub fn new(lynt: Lynt) -> Self {
        let comments_job = Some(spawn_comments_fetch(lynt.clone()));
        Self {
            lynt,
            liked: false,
            like_busy: false,
            like_error: None,
            like_job: None,
            comments: CommentsState::Loading,
            comments_job,
            comment_like_job: None,
            composer: ComposeBox::new(),
            editing_composer: false,
            post_button: Button::new("Post"),
            post_busy: false,
            post_error: None,
            post_job: None,
            hovered: 0,
            action_cursor: 0,
            scroll_offset: 0,
            visible_end: Cell::new(0),
            navigate_to: None,
            navigate_to_profile: None,
            editing_post: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing_composer || self.editing_post.is_some()
    }

    pub fn exit_editing(&mut self) {
        if let Some(edit) = &mut self.editing_post {
            if edit.editing_box {
                edit.editing_box = false;
            } else {
                self.editing_post = None;
            }
            return;
        }
        self.editing_composer = false;
    }

    pub fn take_navigate_to(&mut self) -> Option<Lynt> {
        self.navigate_to.take()
    }

    pub fn take_navigate_to_profile(&mut self) -> Option<String> {
        self.navigate_to_profile.take()
    }

    pub fn tick(&mut self) {
        self.poll_like();
        self.poll_comments();
        self.poll_comment_like();
        self.poll_post();
        self.poll_edit_post();
        self.post_button.tick();
        let count = self.composer.value().trim().chars().count();
        self.post_button.enabled = count > 0 && count <= MAX_COMMENT_CHARS && !self.post_busy;
        self.post_button.busy = self.post_busy;
        self.post_button.label = if self.post_busy {
            "Posting…".to_string()
        } else {
            "Post".to_string()
        };
    }

    fn poll_like(&mut self) {
        let Some(rx) = &self.like_job else { return };
        match rx.try_recv() {
            Ok(Ok(now_liked)) => {
                self.like_busy = false;
                self.like_job = None;
                self.liked = now_liked;
                self.like_error = None;
            }
            Ok(Err(msg)) => {
                self.like_busy = false;
                self.like_job = None;
                self.like_error = Some(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.like_busy = false;
                self.like_job = None;
            }
        }
    }

    fn poll_comments(&mut self) {
        let Some(rx) = &self.comments_job else { return };
        match rx.try_recv() {
            Ok(Ok(comments)) => {
                self.comments_job = None;
                self.comments = CommentsState::Loaded(
                    comments
                        .into_iter()
                        .map(|lynt| CommentEntry {
                            lynt,
                            liked: false,
                            busy: false,
                        })
                        .collect(),
                );
            }
            Ok(Err(msg)) => {
                self.comments_job = None;
                self.comments = CommentsState::Failed(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.comments_job = None;
            }
        }
    }

    fn poll_comment_like(&mut self) {
        let Some(rx) = &self.comment_like_job else {
            return;
        };
        match rx.try_recv() {
            Ok((id, outcome)) => {
                self.comment_like_job = None;
                if let CommentsState::Loaded(comments) = &mut self.comments {
                    if let Some(entry) = comments.iter_mut().find(|e| e.lynt.id == id) {
                        entry.busy = false;
                        if let Ok(liked) = outcome {
                            entry.liked = liked;
                        }
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.comment_like_job = None;
            }
        }
    }

    fn poll_post(&mut self) {
        let Some(rx) = &self.post_job else { return };
        match rx.try_recv() {
            Ok(Ok(new_comment)) => {
                self.post_job = None;
                self.post_busy = false;
                self.composer.set_value("");
                self.post_error = None;
                let entry = CommentEntry {
                    lynt: new_comment,
                    liked: false,
                    busy: false,
                };
                match &mut self.comments {
                    CommentsState::Loaded(comments) => comments.insert(0, entry),
                    _ => self.comments = CommentsState::Loaded(vec![entry]),
                }
            }
            Ok(Err(msg)) => {
                self.post_job = None;
                self.post_busy = false;
                self.post_error = Some(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.post_job = None;
                self.post_busy = false;
            }
        }
    }

    fn poll_edit_post(&mut self) {
        let Some(edit) = &mut self.editing_post else {
            return;
        };
        edit.save_btn.tick();
        edit.save_btn.busy = edit.busy;
        edit.save_btn.enabled = !edit.box_.value().trim().is_empty() && !edit.busy;
        edit.save_btn.label = if edit.busy {
            "Saving…".to_string()
        } else {
            "Save".to_string()
        };

        let Some(rx) = &edit.job else { return };
        let outcome = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                edit.job = None;
                edit.busy = false;
                return;
            }
        };
        edit.job = None;
        edit.busy = false;
        match outcome {
            Ok(new_lynt) => {
                self.lynt = new_lynt;
                self.editing_post = None;
            }
            Err(msg) => {
                if let Some(edit) = &mut self.editing_post {
                    edit.error = Some(msg);
                }
            }
        }
    }

    fn total_hover_slots(&self) -> usize {
        1 + self.scrollable_slot_count()
    }

    fn scrollable_slot_count(&self) -> usize {
        2 + match &self.comments {
            CommentsState::Loaded(c) => c.len(),
            _ => 0,
        }
    }

    fn follow_scroll(&mut self) {
        if self.hovered == 0 {
            return;
        }
        let si = self.hovered - 1;
        if si < self.scroll_offset || si >= self.visible_end.get() {
            self.scroll_offset = si;
        }
    }

    fn hovered_is_lynt_row(&self) -> bool {
        self.hovered == 0 || self.hovered >= 3
    }

    fn hovered_lynt_handle(&self) -> Option<String> {
        if self.hovered == 0 {
            return Some(self.lynt.handle.clone());
        }
        if self.hovered >= 3 {
            if let CommentsState::Loaded(comments) = &self.comments {
                return comments
                    .get(self.hovered - 3)
                    .map(|e| e.lynt.handle.clone());
            }
        }
        None
    }

    fn start_post(&mut self) {
        if self.post_busy {
            return;
        }
        let content = self.composer.value().trim().to_string();
        if content.is_empty() || content.chars().count() > MAX_COMMENT_CHARS {
            return;
        }
        self.post_busy = true;
        self.post_error = None;
        self.post_job = Some(spawn_reply(self.lynt.clone(), content));
    }

    fn toggle_like_hovered(&mut self) {
        match self.hovered {
            0 => {
                if !self.like_busy {
                    self.like_busy = true;
                    self.like_error = None;
                    self.like_job = Some(spawn_like_toggle(self.lynt.clone(), !self.liked));
                }
            }
            h if h >= 3 => self.start_comment_like_toggle(h - 3),
            _ => {}
        }
    }

    fn start_comment_like_toggle(&mut self, idx: usize) {
        if self.comment_like_job.is_some() {
            return;
        }
        let CommentsState::Loaded(comments) = &mut self.comments else {
            return;
        };
        let Some(entry) = comments.get_mut(idx) else {
            return;
        };
        if entry.busy {
            return;
        }
        entry.busy = true;
        let like = !entry.liked;
        self.comment_like_job = Some(spawn_comment_like_toggle(
            entry.lynt.clone(),
            like,
            entry.lynt.id.clone(),
        ));
    }

    fn activate_lynt_row_action(&mut self) {
        if self.action_cursor == 0 {
            self.toggle_like_hovered();
            return;
        }
        if self.hovered == 0 {
            self.hovered = 1;
            self.action_cursor = 0;
            self.editing_composer = true;
        } else if self.hovered >= 3 {
            let ci = self.hovered - 3;
            if let CommentsState::Loaded(comments) = &self.comments {
                if let Some(entry) = comments.get(ci) {
                    self.navigate_to = Some(entry.lynt.clone());
                }
            }
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if let Some(edit) = &mut self.editing_post {
            if edit.editing_box {
                edit.box_.handle_key(code);
                return;
            }
            match code {
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Char('j')
                | KeyCode::Char('J')
                | KeyCode::Char('k')
                | KeyCode::Char('K') => {
                    edit.hovered = !edit.hovered;
                }
                KeyCode::Enter => {
                    if edit.hovered {
                        let content = edit.box_.value().trim().to_string();
                        if !content.is_empty() && !edit.busy {
                            edit.busy = true;
                            edit.error = None;
                            edit.job = Some(spawn_edit(self.lynt.clone(), content));
                        }
                    } else {
                        edit.editing_box = true;
                    }
                }
                _ => {}
            }
            return;
        }

        if self.editing_composer {
            self.composer.handle_key(code);
            return;
        }

        if (code == KeyCode::Char('e') || code == KeyCode::Char('E'))
            && self.hovered == 0
            && self.lynt.user_id == client::my_id()
        {
            let mut edit = EditPostState {
                box_: ComposeBox::new(),
                editing_box: false,
                hovered: false,
                save_btn: Button::new("Save"),
                busy: false,
                error: None,
                job: None,
            };
            edit.box_.set_value(self.lynt.content.clone());
            self.editing_post = Some(edit);
            return;
        }

        if (code == KeyCode::Char('p') || code == KeyCode::Char('P')) && self.hovered_is_lynt_row()
        {
            self.navigate_to_profile = self.hovered_lynt_handle();
            return;
        }

        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.hovered = self.hovered.saturating_sub(1);
                self.action_cursor = 0;
                self.follow_scroll();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.hovered = (self.hovered + 1).min(self.total_hover_slots() - 1);
                self.action_cursor = 0;
                self.follow_scroll();
            }
            KeyCode::Left if self.hovered_is_lynt_row() => self.action_cursor = 0,
            KeyCode::Right if self.hovered_is_lynt_row() => self.action_cursor = 1,
            KeyCode::Enter => {
                if self.hovered_is_lynt_row() {
                    self.activate_lynt_row_action();
                } else if self.hovered == 1 {
                    if !self.composer.locked {
                        self.editing_composer = true;
                    }
                } else if self.hovered == 2 {
                    self.start_post();
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => self.toggle_like_hovered(),
            _ => {}
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        if let Some(edit) = &self.editing_post {
            return if edit.editing_box {
                vec![
                    ("↑/↓".into(), "Line".into()),
                    ("↵".into(), "New Line".into()),
                    ("Esc".into(), "Stop Editing".into()),
                ]
            } else {
                vec![
                    ("↓/↑".into(), "Switch".into()),
                    (
                        "↵".into(),
                        if edit.hovered {
                            "Save".into()
                        } else {
                            "Edit".into()
                        },
                    ),
                    ("Esc".into(), "Cancel".into()),
                ]
            };
        }
        if self.editing_composer {
            return vec![
                ("←/→".into(), "Cursor".into()),
                ("↑/↓".into(), "Line".into()),
                ("↵".into(), "New Line".into()),
                ("Esc".into(), "Stop Editing".into()),
            ];
        }
        let mut hints = vec![("↓/J".into(), "Down".into()), ("↑/K".into(), "Up".into())];
        if self.hovered_is_lynt_row() {
            hints.push(("←/→".into(), "Choose Action".into()));
            hints.push((
                "↵".into(),
                if self.action_cursor == 0 {
                    "Like/Unlike".into()
                } else {
                    "Comment".into()
                },
            ));
            hints.push(("L".into(), "Like/Unlike".into()));
            hints.push(("P".into(), "View Profile".into()));
            if self.hovered == 0 && self.lynt.user_id == client::my_id() {
                hints.push(("E".into(), "Edit Post".into()));
            }
        } else if self.hovered == 1 {
            hints.push(("↵".into(), "Edit".into()));
        } else {
            hints.push(("↵".into(), "Press".into()));
        }
        hints.push(("Esc".into(), "Close".into()));
        hints
    }

    fn scrollable_slot_height(&self, si: usize, width: u16) -> u16 {
        match si {
            0 => COMPOSER_HEIGHT,
            1 => POST_BUTTON_HEIGHT,
            i => match &self.comments {
                CommentsState::Loaded(comments) => {
                    let is_hovered = self.hovered == i + 1;
                    comments
                        .get(i - 2)
                        .map(|entry| {
                            LyntView::new(&entry.lynt)
                                .hovered(is_hovered)
                                .height(width, MAX_COMMENT_LINES)
                        })
                        .unwrap_or(0)
                }
                _ => 0,
            },
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        if let Some(edit) = &self.editing_post {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Length(1),
                    Constraint::Length(3),
                ])
                .split(area);
            let hover = focused && !edit.editing_box && !edit.hovered;
            let editing = focused && edit.editing_box;
            edit.box_.draw(frame, rows[0], hover, editing, "Edit Lynt");
            if let Some(err) = &edit.error {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't save: {err}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[1],
                );
            }
            edit.save_btn.draw(frame, rows[2], focused && edit.hovered);
            return;
        }

        let top_action_focus = if focused && self.hovered == 0 {
            Some(self.action_cursor)
        } else {
            None
        };
        let view = LyntView::new(&self.lynt)
            .liked(self.liked)
            .busy(self.like_busy)
            .action_focus(top_action_focus);
        let max_post_lines = area.height.saturating_sub(8).max(3);
        let post_height = view.height(area.width, max_post_lines);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(post_height),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        view.draw(frame, rows[0]);

        if let Some(err) = &self.like_error {
            frame.render_widget(
                Paragraph::new(format!("Couldn't update like: {err}"))
                    .style(Style::default().fg(Color::Red)),
                rows[1],
            );
        }

        let hbar = "─".repeat(rows[2].width as usize);
        frame.render_widget(
            Paragraph::new(hbar).style(Style::default().fg(Color::DarkGray)),
            rows[2],
        );

        match &self.comments {
            CommentsState::Loading => {
                frame.render_widget(Paragraph::new("Loading comments…"), rows[3]);
            }
            CommentsState::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load comments: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[3],
                );
            }
            CommentsState::Loaded(comments) => {
                let label = if comments.is_empty() {
                    "No comments yet.".to_string()
                } else {
                    format!(
                        "{} comment{}",
                        comments.len(),
                        if comments.len() == 1 { "" } else { "s" }
                    )
                };
                frame.render_widget(Paragraph::new(label), rows[3]);
            }
        }

        let mut y = 0u16;
        let mut rendered_end = self.scroll_offset;
        for si in self.scroll_offset..self.scrollable_slot_count() {
            let h = self.scrollable_slot_height(si, rows[4].width);
            if h == 0 || y + h > rows[4].height {
                break;
            }
            let item_area = Rect {
                x: rows[4].x,
                y: rows[4].y + y,
                width: rows[4].width,
                height: h,
            };
            match si {
                0 => {
                    let box_rows = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(5), Constraint::Length(1)])
                        .split(item_area);
                    let hover = focused && !self.editing_composer && self.hovered == 1;
                    let editing = focused && self.editing_composer && self.hovered == 1;
                    self.composer
                        .draw(frame, box_rows[0], hover, editing, "Comment");

                    let count = self.composer.value().chars().count();
                    let (status, color) = if count > MAX_COMMENT_CHARS {
                        (
                            format!("{count} / {MAX_COMMENT_CHARS} — too long"),
                            Color::Red,
                        )
                    } else {
                        (format!("{count} / {MAX_COMMENT_CHARS}"), Color::DarkGray)
                    };
                    frame.render_widget(
                        Paragraph::new(status).style(Style::default().fg(color)),
                        box_rows[1],
                    );

                    if let Some(err) = &self.post_error {
                        frame.render_widget(
                            Paragraph::new(format!("Couldn't post: {err}"))
                                .style(Style::default().fg(Color::Red)),
                            box_rows[1],
                        );
                    }
                }
                1 => {
                    self.post_button
                        .draw(frame, item_area, focused && self.hovered == 2);
                }
                i => {
                    if let CommentsState::Loaded(comments) = &self.comments {
                        if let Some(entry) = comments.get(i - 2) {
                            let row_index = i + 1;
                            let is_hovered = focused && self.hovered == row_index;
                            let action_focus = if is_hovered {
                                Some(self.action_cursor)
                            } else {
                                None
                            };
                            LyntView::new(&entry.lynt)
                                .liked(entry.liked)
                                .busy(entry.busy)
                                .hovered(is_hovered)
                                .action_focus(action_focus)
                                .draw(frame, item_area);
                        }
                    }
                }
            }
            y += h;
            rendered_end = si + 1;
        }
        self.visible_end.set(rendered_end.max(self.scroll_offset));
    }
}
