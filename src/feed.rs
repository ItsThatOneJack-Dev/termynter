/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Lynt;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::widgets::{LyntList, LyntListAction};

const AUTO_RELOAD_INTERVAL: Duration = Duration::from_secs(30);

fn spawn_feed_fetch(kind: &'static str) -> Receiver<Result<Vec<Lynt>, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { client.feed(kind).await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

#[derive(Clone, Copy, PartialEq)]
enum FeedKind {
    New,
    Following,
    ForYou,
}

impl FeedKind {
    fn label(self) -> &'static str {
        match self {
            FeedKind::New => "New",
            FeedKind::Following => "Following",
            FeedKind::ForYou => "For You",
        }
    }
    fn api_kind(self) -> &'static str {
        match self {
            FeedKind::New => "New",
            FeedKind::Following => "Following",
            FeedKind::ForYou => "For you",
        }
    }
}

const KINDS: [FeedKind; 3] = [FeedKind::New, FeedKind::Following, FeedKind::ForYou];

#[derive(Clone, Copy)]
enum JobMode {
    Switch,
    Reload,
}

enum State {
    Loading,
    Loaded(LyntList),
    Failed(String),
}

#[derive(PartialEq)]
enum FeedFocus {
    Tabs,
    List,
}

pub struct FeedPage {
    state: State,
    job: Option<(JobMode, Receiver<Result<Vec<Lynt>, String>>)>,
    active_kind: FeedKind,
    hovered_tab: usize,
    sub_focus: FeedFocus,
    reload_busy: bool,
    last_reload: Instant,
    action: Option<LyntListAction>,
}

impl FeedPage {
    pub fn new() -> Self {
        Self {
            state: State::Loading,
            job: Some((JobMode::Switch, spawn_feed_fetch(FeedKind::New.api_kind()))),
            active_kind: FeedKind::New,
            hovered_tab: 0,
            sub_focus: FeedFocus::List,
            reload_busy: false,
            last_reload: Instant::now(),
            action: None,
        }
    }

    pub fn tick(&mut self) {
        self.poll_job();
        if self.job.is_none() && self.last_reload.elapsed() >= AUTO_RELOAD_INTERVAL {
            self.start_reload();
        }
    }

    fn poll_job(&mut self) {
        let Some((mode, rx)) = &self.job else { return };
        let mode = *mode;
        match rx.try_recv() {
            Ok(Ok(lynts)) => {
                self.job = None;
                self.reload_busy = false;
                self.last_reload = Instant::now();
                match mode {
                    JobMode::Switch => self.state = State::Loaded(LyntList::new(lynts)),
                    JobMode::Reload => match &mut self.state {
                        State::Loaded(list) => list.replace(lynts),
                        _ => self.state = State::Loaded(LyntList::new(lynts)),
                    },
                }
            }
            Ok(Err(msg)) => {
                self.job = None;
                self.reload_busy = false;
                self.last_reload = Instant::now();
                match mode {
                    JobMode::Switch => self.state = State::Failed(msg),
                    JobMode::Reload => {}
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.reload_busy = false;
            }
        }
    }

    fn seconds_until_reload(&self) -> u64 {
        AUTO_RELOAD_INTERVAL
            .saturating_sub(self.last_reload.elapsed())
            .as_secs()
    }

    pub fn take_action(&mut self) -> Option<LyntListAction> {
        self.action.take()
    }

    fn start_switch(&mut self, kind: FeedKind) {
        if self.job.is_some() {
            return;
        }
        self.active_kind = kind;
        self.state = State::Loading;
        self.job = Some((JobMode::Switch, spawn_feed_fetch(kind.api_kind())));
    }

    fn start_reload(&mut self) {
        if self.job.is_some() {
            return;
        }
        self.reload_busy = true;
        self.job = Some((
            JobMode::Reload,
            spawn_feed_fetch(self.active_kind.api_kind()),
        ));
    }

    pub fn handle_shift_vertical(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.sub_focus = FeedFocus::Tabs,
            KeyCode::Down => self.sub_focus = FeedFocus::List,
            _ => {}
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match self.sub_focus {
            FeedFocus::Tabs => match code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.hovered_tab = self.hovered_tab.saturating_sub(1);
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.hovered_tab = (self.hovered_tab + 1).min(3);
                }
                KeyCode::Enter => {
                    if self.hovered_tab == 3 {
                        self.start_reload();
                    } else {
                        let kind = KINDS[self.hovered_tab];
                        if kind != self.active_kind {
                            self.start_switch(kind);
                        }
                    }
                }
                _ => {}
            },
            FeedFocus::List => {
                if let State::Loaded(list) = &mut self.state {
                    if let Some(action) = list.handle_key(code) {
                        self.action = Some(action);
                    }
                }
            }
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        match self.sub_focus {
            FeedFocus::Tabs => vec![
                ("←/H".into(), "Left".into()),
                ("→/L".into(), "Right".into()),
                (
                    "↵".into(),
                    if self.hovered_tab == 3 {
                        "Reload".into()
                    } else {
                        "Press".into()
                    },
                ),
                ("⇧+↓".into(), "To List".into()),
            ],
            FeedFocus::List => {
                let mut hints = vec![("↓/J".into(), "Down".into()), ("↑/K".into(), "Up".into())];
                if matches!(&self.state, State::Loaded(list) if !list.is_empty()) {
                    hints.push(("↵".into(), "Open".into()));
                    hints.push(("P".into(), "View Profile".into()));
                }
                hints.push(("⇧+↑".into(), "To Tabs".into()));
                hints
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, content_focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        self.draw_tabs(
            frame,
            rows[0],
            content_focused && self.sub_focus == FeedFocus::Tabs,
        );
        let divider = "─".repeat(rows[1].width as usize);
        frame.render_widget(
            Paragraph::new(divider).style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );
        match &self.state {
            State::Loading => {
                frame.render_widget(Paragraph::new("Loading feed…"), rows[2]);
            }
            State::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Couldn't load feed: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[2],
                );
            }
            State::Loaded(list) => list.draw(
                frame,
                rows[2],
                content_focused && self.sub_focus == FeedFocus::List,
            ),
        }
    }

    fn draw_tabs(&self, frame: &mut Frame, area: Rect, tabs_focused: bool) {
        let mut spans = Vec::new();
        for (i, kind) in KINDS.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            let is_active = *kind == self.active_kind;
            let is_hovered = tabs_focused && self.hovered_tab == i;
            let mut style = Style::default();
            if is_active {
                style = style.add_modifier(Modifier::BOLD).fg(Color::Cyan);
            }
            if is_hovered {
                style = style.fg(Color::Yellow);
            }
            let label = if is_hovered {
                format!("[{}]", kind.label())
            } else {
                kind.label().to_string()
            };
            spans.push(Span::styled(label, style));
        }
        spans.push(Span::raw("    "));
        let reload_hovered = tabs_focused && self.hovered_tab == 3;
        let reload_label = if self.reload_busy {
            "Reloading…".to_string()
        } else {
            "Reload".to_string()
        };
        let mut reload_style = Style::default();
        if reload_hovered {
            reload_style = reload_style.fg(Color::Yellow);
        }
        let reload_text = if reload_hovered {
            format!("[{reload_label}]")
        } else {
            reload_label
        };
        spans.push(Span::styled(reload_text, reload_style));
        if !self.reload_busy {
            let secs = self.seconds_until_reload();
            let unit = if secs == 1 { "second" } else { "seconds" };
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("Reloading in {secs} {unit}."),
                Style::default().fg(Color::DarkGray),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}
