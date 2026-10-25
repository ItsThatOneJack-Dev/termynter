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

use crate::widgets::TextInput;

fn spawn_fetch(id: String) -> Receiver<Result<Lynt, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { client.lynt(&id).await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

pub struct LyntLookup {
    id_input: TextInput,
    editing: bool,
    fetched: Option<Lynt>,
    error: Option<String>,
    loading: bool,
    job: Option<Receiver<Result<Lynt, String>>>,
}

impl LyntLookup {
    pub fn new() -> Self {
        Self {
            id_input: TextInput::new(),
            editing: false,
            fetched: None,
            error: None,
            loading: false,
            job: None,
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
            Ok(Ok(lynt)) => {
                self.job = None;
                self.loading = false;
                self.id_input.locked = false;
                self.error = None;
                self.fetched = Some(lynt);
            }
            Ok(Err(msg)) => {
                self.job = None;
                self.loading = false;
                self.id_input.locked = false;
                self.fetched = None;
                self.error = Some(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.loading = false;
                self.id_input.locked = false;
                self.error = Some("Background fetch thread disconnected.".to_string());
            }
        }
    }

    fn start_fetch(&mut self) {
        let id = self.id_input.value().trim().to_string();
        if self.loading || id.is_empty() {
            return;
        }
        self.loading = true;
        self.id_input.locked = true;
        self.error = None;
        self.job = Some(spawn_fetch(id));
    }

    /// Returns the fetched lynt once, if one's waiting to be opened.
    pub fn take_fetched(&mut self) -> Option<Lynt> {
        self.fetched.take()
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if self.editing {
            if code == KeyCode::Enter {
                self.editing = false;
                self.start_fetch();
                return;
            }
            if code == KeyCode::Esc {
                self.editing = false;
                return;
            }
            self.id_input.handle_key(code);
            return;
        }
        if code == KeyCode::Enter {
            self.editing = true;
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        if self.editing {
            vec![
                ("←".into(), "Cursor Left".into()),
                ("→".into(), "Cursor Right".into()),
                ("↵".into(), "Fetch".into()),
                ("Esc".into(), "Stop Editing".into()),
            ]
        } else {
            vec![("↵".into(), "Edit".into())]
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(area);

        self.id_input.draw(
            frame,
            rows[0],
            "Lynt ID",
            focused && !self.editing,
            focused && self.editing,
        );

        if self.loading {
            frame.render_widget(Paragraph::new("Loading…"), rows[1]);
        } else if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(format!("Error: {err}")).style(Style::default().fg(Color::Red)),
                rows[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new("Enter a lynt id and press Enter.")
                    .style(Style::default().fg(Color::DarkGray)),
                rows[1],
            );
        }
    }
}
