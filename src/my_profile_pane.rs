/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::{Lynt, Me};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::widgets::{LyntList, LyntListAction, ProfileRef, profile};

const PROFILE_SECTION_HEIGHT: u16 = 16;

fn spawn_own_posts_fetch(handle: String) -> Receiver<Result<Vec<Lynt>, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        // `lyntr-rs` has no "posts by user" endpoint, though that is a pretty critical feature, so it is approximated here by running a search for the user's handle then filtered to only posts posted by the actual user.
        let result = runtime.block_on(async {
            let results = client.search(&handle).await.map_err(|e| e.to_string())?;
            Ok::<Vec<Lynt>, String>(results.into_iter().filter(|l| l.handle == handle).collect())
        });
        let _ = tx.send(result);
    });
    rx
}

enum PostsState {
    Loading,
    Loaded(LyntList),
    Failed(String),
}

#[derive(PartialEq)]
enum ProfileFocus {
    Header,
    Posts,
}

pub struct MyProfilePane {
    posts: PostsState,
    job: Option<Receiver<Result<Vec<Lynt>, String>>>,
    sub_focus: ProfileFocus,
    pub open_edit: bool,
    action: Option<LyntListAction>,
}

impl MyProfilePane {
    pub fn new(handle: String) -> Self {
        Self {
            posts: PostsState::Loading,
            job: Some(spawn_own_posts_fetch(handle)),
            sub_focus: ProfileFocus::Header,
            open_edit: false,
            action: None,
        }
    }

    pub fn tick(&mut self) {
        let Some(rx) = &self.job else { return };
        match rx.try_recv() {
            Ok(Ok(posts)) => {
                self.job = None;
                self.posts = PostsState::Loaded(LyntList::new(posts));
            }
            Ok(Err(msg)) => {
                self.job = None;
                self.posts = PostsState::Failed(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.job = None;
            }
        }
    }

    pub fn take_action(&mut self) -> Option<LyntListAction> {
        self.action.take()
    }

    pub fn handle_shift_vertical(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.sub_focus = ProfileFocus::Header,
            KeyCode::Down => self.sub_focus = ProfileFocus::Posts,
            _ => {}
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match self.sub_focus {
            ProfileFocus::Header => {
                if code == KeyCode::Enter {
                    self.open_edit = true;
                }
            }
            ProfileFocus::Posts => {
                if let PostsState::Loaded(list) = &mut self.posts {
                    if let Some(action) = list.handle_key(code) {
                        self.action = Some(action);
                    }
                }
            }
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        match self.sub_focus {
            ProfileFocus::Header => vec![
                ("↵".into(), "Edit Profile".into()),
                ("⇧+↓".into(), "To Posts".into()),
            ],
            ProfileFocus::Posts => {
                let mut hints = vec![("↓/J".into(), "Down".into()), ("↑/K".into(), "Up".into())];
                if matches!(&self.posts, PostsState::Loaded(list) if !list.is_empty()) {
                    hints.push(("↵".into(), "Open".into()));
                }
                hints.push(("⇧+↑".into(), "To Profile".into()));
                hints
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, me: &Me, focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(PROFILE_SECTION_HEIGHT),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        let header_focused = focused && self.sub_focus == ProfileFocus::Header;
        profile::draw(frame, rows[0], ProfileRef::Mine(me), header_focused);

        let divider = "─".repeat(rows[1].width as usize);
        frame.render_widget(
            Paragraph::new(divider).style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );

        frame.render_widget(Paragraph::new("Your Lynts"), rows[2]);

        let posts_focused = focused && self.sub_focus == ProfileFocus::Posts;
        match &self.posts {
            PostsState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading your lynts…")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[3],
                );
            }
            PostsState::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load your lynts: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[3],
                );
            }
            PostsState::Loaded(list) => list.draw(frame, rows[3], posts_focused),
        }
    }
}
