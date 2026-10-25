/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::{LyntrClient, Me};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::widgets::{Button, TextInput, Toast, ToastStack};

fn spawn_check(
    client_id: String,
    client_secret: String,
) -> Receiver<Result<(LyntrClient, Me), String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            let client = LyntrClient::new(&client_id, &client_secret)
                .await
                .map_err(|e| e.to_string())?;
            let me = client.me().await.map_err(|e| e.to_string())?;
            Ok((client, me))
        });
        let _ = tx.send(result);
    });
    rx
}

#[derive(PartialEq, Clone, Copy)]
enum FormItem {
    ClientId,
    ClientSecret,
    Submit,
}

const FORM_ITEMS: [FormItem; 3] = [FormItem::ClientId, FormItem::ClientSecret, FormItem::Submit];

pub struct LoginPage {
    pub client_id: TextInput,
    pub client_secret: TextInput,
    hovered: usize,
    editing: bool,
    submit: Button,
    pub toasts: ToastStack,
    verified: Option<(LyntrClient, Me)>,
    job: Option<Receiver<Result<(LyntrClient, Me), String>>>,
    pub completed: Option<Me>,
}

impl LoginPage {
    pub fn new() -> Self {
        let mut page = Self {
            client_id: TextInput::new(),
            client_secret: TextInput::new(),
            hovered: 0,
            editing: false,
            submit: Button::new("Check"),
            toasts: ToastStack::default(),
            verified: None,
            job: None,
            completed: None,
        };
        page.client_id.set_validation(validate_client_id(""));
        page.client_secret
            .set_validation(validate_client_secret(""));
        page.refresh_submit_enabled();
        page
    }

    fn hovered_item(&self) -> FormItem {
        FORM_ITEMS[self.hovered]
    }

    fn refresh_submit_enabled(&mut self) {
        self.submit.enabled = self.client_id.is_valid() && self.client_secret.is_valid();
    }

    // TODO: DEAD CODE CLEANUP
    // pub fn is_editing(&self) -> bool {
    //     self.editing
    // }

    pub fn exit_editing(&mut self) {
        self.editing = false;
    }

    pub fn tick(&mut self) {
        self.submit.tick();
        self.toasts.tick();
        self.poll_check();
    }

    fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn lock(&mut self, locked: bool) {
        self.client_id.locked = locked;
        self.client_secret.locked = locked;
        self.submit.busy = locked;
    }

    fn poll_check(&mut self) {
        let Some(rx) = &self.job else { return };
        match rx.try_recv() {
            Ok(Ok((client, me))) => {
                self.lock(false);
                self.job = None;
                self.verified = Some((client, me));
                self.submit.label = "Use This Credential".to_string();
                self.toasts.push(Toast::new(
                    "This credential is valid, press enter again to use it.",
                ));
            }
            Ok(Err(msg)) => {
                self.lock(false);
                self.job = None;
                self.verified = None;
                self.submit.label = "Check".to_string();
                self.toasts
                    .push(Toast::new(format!("This credential is not valid: {msg}")));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.lock(false);
                self.job = None;
            }
        }
    }

    fn start_check(&mut self) {
        if self.busy() || !self.client_id.is_valid() || !self.client_secret.is_valid() {
            return;
        }
        self.lock(true);
        self.job = Some(spawn_check(
            self.client_id.value().to_string(),
            self.client_secret.value().to_string(),
        ));
    }

    fn start_use(&mut self) {
        if let Some((client, me)) = self.verified.take() {
            let client_id = self.client_id.value().to_string();
            crate::client::set(client, me.user.id.clone(), client_id);
            self.completed = Some(me);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, kind: KeyEventKind) {
        if kind != KeyEventKind::Press {
            return;
        }

        if self.editing {
            if code == KeyCode::Enter || code == KeyCode::Esc {
                self.editing = false;
                return;
            }
            let changed = match self.hovered_item() {
                FormItem::ClientId => {
                    let changed = self.client_id.handle_key(code);
                    if changed {
                        self.client_id
                            .set_validation(validate_client_id(self.client_id.value()));
                    }
                    changed
                }
                FormItem::ClientSecret => {
                    let changed = self.client_secret.handle_key(code);
                    if changed {
                        self.client_secret
                            .set_validation(validate_client_secret(self.client_secret.value()));
                    }
                    changed
                }
                FormItem::Submit => false,
            };
            if changed {
                self.refresh_submit_enabled();
                if self.verified.is_some() {
                    self.verified = None;
                    self.submit.label = "Check".to_string();
                }
            }
            return;
        }

        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.hovered = self.hovered.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.hovered = (self.hovered + 1).min(FORM_ITEMS.len() - 1);
            }
            KeyCode::Enter => match self.hovered_item() {
                FormItem::Submit => {
                    if self.verified.is_some() {
                        self.start_use();
                    } else {
                        self.start_check();
                    }
                }
                _ => self.editing = true,
            },
            _ => {}
        }
    }

    pub fn hints(&self, _shift_held: bool) -> (Vec<(String, String)>, bool) {
        if self.editing {
            return (
                vec![
                    ("←".into(), "Cursor Left".into()),
                    ("→".into(), "Cursor Right".into()),
                    ("↵".into(), "Confirm".into()),
                    ("Esc".into(), "Stop Editing".into()),
                ],
                false,
            );
        }
        let edit_or_press = if self.hovered_item() == FormItem::Submit {
            "Press"
        } else {
            "Edit"
        };
        (
            vec![
                ("↓/J".into(), "Hover Down".into()),
                ("↑/K".into(), "Hover Up".into()),
                ("↵".into(), edit_or_press.into()),
            ],
            false,
        )
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title("Sign In");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(inner);

        let id_hover = self.hovered_item() == FormItem::ClientId && !self.editing;
        let id_editing = self.hovered_item() == FormItem::ClientId && self.editing;
        self.client_id
            .draw(frame, rows[0], "Client ID", id_hover, id_editing);
        if let Some(msg) = self.client_id.validation_message() {
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::Red)),
                rows[1],
            );
        }

        let secret_hover = self.hovered_item() == FormItem::ClientSecret && !self.editing;
        let secret_editing = self.hovered_item() == FormItem::ClientSecret && self.editing;
        self.client_secret.draw(
            frame,
            rows[2],
            "Client Secret",
            secret_hover,
            secret_editing,
        );
        if let Some(msg) = self.client_secret.validation_message() {
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::Red)),
                rows[3],
            );
        }

        self.submit
            .draw(frame, rows[5], self.hovered_item() == FormItem::Submit);
    }
}

fn validate_client_id(text: &str) -> Option<String> {
    if text.is_empty() {
        Some("Enter a client ID.".into())
    } else if text.len() != 38 {
        Some("Too short, all client IDs are 47 characters in length.".into())
    } else if text.contains(' ') {
        Some("Client IDs cannot contain spaces.".into())
    } else if !text.starts_with("lyntr_") {
        Some("All client IDs start with \"lyntr_\".".into())
    } else {
        None
    }
}

fn validate_client_secret(text: &str) -> Option<String> {
    if text.is_empty() {
        Some("Enter an client secret.".into())
    } else if text.len() != 47 {
        Some("Too short, all client secrets are 47 characters in length.".into())
    } else if text.contains(' ') {
        Some("Client secrets cannot contain spaces.".into())
    } else if !text.starts_with("lsk_") {
        Some("All client secrets start with \"lsk_\".".into())
    } else {
        None
    }
}
