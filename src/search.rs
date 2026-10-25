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

use crate::widgets::{LyntList, LyntListAction, TextInput};

fn spawn_search(query: String) -> Receiver<Result<Vec<Lynt>, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { client.search(&query).await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

enum State {
    Idle,
    Loading,
    Loaded(LyntList),
    Failed(String),
}

#[derive(PartialEq)]
enum SearchFocus {
    Query,
    List,
}

pub struct SearchPage {
    query: TextInput,
    editing: bool,
    sub_focus: SearchFocus,
    state: State,
    job: Option<Receiver<Result<Vec<Lynt>, String>>>,
    action: Option<LyntListAction>,
}

impl SearchPage {
    pub fn new() -> Self {
        Self {
            query: TextInput::new(),
            editing: false,
            sub_focus: SearchFocus::Query,
            state: State::Idle,
            job: None,
            action: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn exit_editing(&mut self) {
        self.editing = false;
    }

    pub fn tick(&mut self) {
        self.poll_job();
    }

    fn poll_job(&mut self) {
        let Some(rx) = &self.job else { return };
        match rx.try_recv() {
            Ok(Ok(lynts)) => {
                self.job = None;
                self.state = State::Loaded(LyntList::new(lynts));
            }
            Ok(Err(msg)) => {
                self.job = None;
                self.state = State::Failed(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.job = None;
            }
        }
    }

    fn start_search(&mut self) {
        let query = self.query.value().trim().to_string();
        if self.job.is_some() || query.is_empty() {
            return;
        }
        self.state = State::Loading;
        self.job = Some(spawn_search(query));
    }

    pub fn take_action(&mut self) -> Option<LyntListAction> {
        self.action.take()
    }

    pub fn handle_shift_vertical(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.sub_focus = SearchFocus::Query,
            KeyCode::Down => self.sub_focus = SearchFocus::List,
            _ => {}
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match self.sub_focus {
            SearchFocus::Query => {
                if self.editing {
                    if code == KeyCode::Enter {
                        self.editing = false;
                        self.start_search();
                    } else if code == KeyCode::Esc {
                        self.editing = false;
                    } else {
                        self.query.handle_key(code);
                    }
                } else if code == KeyCode::Enter {
                    self.editing = true;
                }
            }
            SearchFocus::List => {
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
            SearchFocus::Query if self.editing => vec![
                ("←".into(), "Cursor Left".into()),
                ("→".into(), "Cursor Right".into()),
                ("↵".into(), "Search".into()),
                ("Esc".into(), "Stop Editing".into()),
            ],
            SearchFocus::Query => vec![
                ("↵".into(), "Edit".into()),
                ("⇧+↓".into(), "To Results".into()),
            ],
            SearchFocus::List => {
                let mut hints = vec![("↓/J".into(), "Down".into()), ("↑/K".into(), "Up".into())];
                if matches!(&self.state, State::Loaded(list) if !list.is_empty()) {
                    hints.push(("↵".into(), "Open".into()));
                    hints.push(("P".into(), "View Profile".into()));
                }
                hints.push(("⇧+↑".into(), "To Search".into()));
                hints
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        let query_hover = focused && self.sub_focus == SearchFocus::Query && !self.editing;
        let query_editing = focused && self.sub_focus == SearchFocus::Query && self.editing;
        self.query
            .draw(frame, rows[0], "Search", query_hover, query_editing);
        let list_focused = focused && self.sub_focus == SearchFocus::List;
        match &self.state {
            State::Idle => {
                frame.render_widget(
                    Paragraph::new("Type a query above and press Enter to search.")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[2],
                );
            }
            State::Loading => {
                frame.render_widget(Paragraph::new("Searching…"), rows[2]);
            }
            State::Failed(msg) => {
                frame.render_widget(
                    Paragraph::new(format!("Search failed: {msg}"))
                        .style(Style::default().fg(Color::Red)),
                    rows[2],
                );
            }
            State::Loaded(list) => list.draw(frame, rows[2], list_focused),
        }
    }
}
