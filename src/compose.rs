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

use crate::widgets::{Button, ComposeBox};

const MAX_CHARS: usize = 280;

fn spawn_post(content: String) -> Receiver<Result<Lynt, String>> {
    let (tx, rx) = mpsc::channel();
    let client = crate::client::get();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async { client.post(&content).await });
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
    rx
}

enum FormItem {
    Box_,
    Post,
}

pub struct ComposePage {
    body: ComposeBox,
    editing: bool,
    hovered: FormItem,
    post_btn: Button,
    busy: bool,
    error: Option<String>,
    job: Option<Receiver<Result<Lynt, String>>>,
}

impl ComposePage {
    pub fn new() -> Self {
        Self {
            body: ComposeBox::new(),
            editing: false,
            hovered: FormItem::Box_,
            post_btn: Button::new("Post"),
            busy: false,
            error: None,
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
        self.post_btn.tick();
        if let Some(rx) = &self.job {
            match rx.try_recv() {
                Ok(Ok(_)) => {
                    self.job = None;
                    self.busy = false;
                    self.error = None;
                    self.body.set_value("");
                }
                Ok(Err(msg)) => {
                    self.job = None;
                    self.busy = false;
                    self.error = Some(msg);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.job = None;
                    self.busy = false;
                }
            }
        }
        let count = self.body.value().trim().chars().count();
        self.post_btn.enabled = count > 0 && count <= MAX_CHARS && !self.busy;
        self.post_btn.busy = self.busy;
        self.post_btn.label = if self.busy {
            "Posting…".to_string()
        } else {
            "Post".to_string()
        };
    }

    fn start_post(&mut self) {
        let content = self.body.value().trim().to_string();
        if self.busy || content.is_empty() || content.chars().count() > MAX_CHARS {
            return;
        }
        self.busy = true;
        self.error = None;
        self.job = Some(spawn_post(content));
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if self.editing {
            if code == KeyCode::Esc {
                self.editing = false;
            } else {
                self.body.handle_key(code);
            }
            return;
        }
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => self.hovered = FormItem::Box_,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.hovered = FormItem::Post
            }
            KeyCode::Enter => match self.hovered {
                FormItem::Box_ => self.editing = true,
                FormItem::Post => self.start_post(),
            },
            _ => {}
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        if self.editing {
            vec![
                ("↑/↓".into(), "Line".into()),
                ("↵".into(), "New Line".into()),
                ("Esc".into(), "Stop Editing".into()),
            ]
        } else {
            match self.hovered {
                FormItem::Box_ => vec![("↓/J".into(), "Down".into()), ("↵".into(), "Edit".into())],
                FormItem::Post => vec![("↑/K".into(), "Up".into()), ("↵".into(), "Press".into())],
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(area);
        let box_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Length(1)])
            .split(rows[0]);

        let hover = focused && !self.editing && matches!(self.hovered, FormItem::Box_);
        let editing = focused && self.editing;
        self.body
            .draw(frame, box_rows[0], hover, editing, "New Lynt");

        let count = self.body.value().chars().count();
        let (status, color) = if count > MAX_CHARS {
            (format!("{count} / {MAX_CHARS} — too long"), Color::Red)
        } else {
            (format!("{count} / {MAX_CHARS}"), Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(color)),
            box_rows[1],
        );

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(format!("Couldn't post: {err}"))
                    .style(Style::default().fg(Color::Red)),
                rows[1],
            );
        }

        self.post_btn.draw(
            frame,
            rows[2],
            focused && matches!(self.hovered, FormItem::Post),
        );
    }
}
