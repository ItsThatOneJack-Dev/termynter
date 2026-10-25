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
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::widgets::{LyntList, LyntListAction, OtherProfile, ProfileRef, profile};

const PROFILE_SECTION_HEIGHT: u16 = 16;

fn spawn_profile_fetch(handle: String) -> Receiver<Result<OtherProfile, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            let profile = client.user(&handle).await.map_err(|e| e.to_string())?;
            Ok(OtherProfile {
                username: profile.username,
                handle: profile.handle,
                bio: profile.bio,
                iq: profile.iq,
                verified: profile.verified,
                contributor: profile.contributor,
                follower_count: profile.follower_count,
                following_count: profile.following_count,
                created_at: profile.created_at,
            })
        });
        let _ = tx.send(result);
    });
    rx
}

fn spawn_posts_fetch(handle: String) -> Receiver<Result<Vec<Lynt>, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            let results = client.search(&handle).await.map_err(|e| e.to_string())?;
            Ok::<Vec<Lynt>, String>(results.into_iter().filter(|l| l.handle == handle).collect())
        });
        let _ = tx.send(result);
    });
    rx
}

enum ProfileState {
    Loading,
    Loaded(OtherProfile),
    Failed(String),
}

enum PostsState {
    Loading,
    Loaded(LyntList),
    Failed(String),
}

pub struct OtherProfilePage {
    profile: ProfileState,
    profile_job: Option<Receiver<Result<OtherProfile, String>>>,
    posts: PostsState,
    posts_job: Option<Receiver<Result<Vec<Lynt>, String>>>,
    action: Option<LyntListAction>,
}

impl OtherProfilePage {
    pub fn new(handle: String) -> Self {
        Self {
            profile: ProfileState::Loading,
            profile_job: Some(spawn_profile_fetch(handle.clone())),
            posts: PostsState::Loading,
            posts_job: Some(spawn_posts_fetch(handle)),
            action: None,
        }
    }

    pub fn tick(&mut self) {
        if let Some(rx) = &self.profile_job {
            match rx.try_recv() {
                Ok(Ok(p)) => {
                    self.profile_job = None;
                    self.profile = ProfileState::Loaded(p);
                }
                Ok(Err(msg)) => {
                    self.profile_job = None;
                    self.profile = ProfileState::Failed(msg);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.profile_job = None;
                }
            }
        }
        if let Some(rx) = &self.posts_job {
            match rx.try_recv() {
                Ok(Ok(posts)) => {
                    self.posts_job = None;
                    self.posts = PostsState::Loaded(LyntList::new(posts));
                }
                Ok(Err(msg)) => {
                    self.posts_job = None;
                    self.posts = PostsState::Failed(msg);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.posts_job = None;
                }
            }
        }
    }

    pub fn take_action(&mut self) -> Option<LyntListAction> {
        self.action.take()
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if let PostsState::Loaded(list) = &mut self.posts {
            if let Some(action) = list.handle_key(code) {
                self.action = Some(action);
            }
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        let mut hints = vec![("↓/J".into(), "Down".into()), ("↑/K".into(), "Up".into())];
        if matches!(&self.posts, PostsState::Loaded(list) if !list.is_empty()) {
            hints.push(("↵".into(), "Open".into()));
        }
        hints
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(PROFILE_SECTION_HEIGHT),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        match &self.profile {
            ProfileState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading profile…").style(Style::default().fg(Color::DarkGray)),
                    rows[0],
                );
            }
            ProfileState::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load profile: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[0],
                );
            }
            ProfileState::Loaded(p) => profile::draw(frame, rows[0], ProfileRef::Other(p), false),
        }

        let divider = "─".repeat(rows[1].width as usize);
        frame.render_widget(
            Paragraph::new(divider).style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );

        frame.render_widget(Paragraph::new("Lynts"), rows[2]);

        match &self.posts {
            PostsState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading lynts…").style(Style::default().fg(Color::DarkGray)),
                    rows[3],
                );
            }
            PostsState::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load lynts: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[3],
                );
            }
            PostsState::Loaded(list) => list.draw(frame, rows[3], focused),
        }
    }
}
