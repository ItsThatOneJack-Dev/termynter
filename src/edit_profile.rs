/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::Me;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::widgets::{Button, TextInput};

fn spawn_update(
    mut me: Me,
    username: String,
    bio: String,
    name_color: Option<String>,
) -> Receiver<(Me, Result<(), String>)> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let outcome = runtime.block_on(async {
            me.user.username = username;
            me.user.bio = bio;
            me.user.name_color = name_color;
            me.update().await.map_err(|e| e.to_string())
        });
        let _ = tx.send((me, outcome));
    });
    rx
}

#[derive(PartialEq, Clone, Copy)]
enum FormItem {
    Username,
    Bio,
    NameColor,
    Update,
}

const FORM_ITEMS: [FormItem; 4] = [
    FormItem::Username,
    FormItem::Bio,
    FormItem::NameColor,
    FormItem::Update,
];

pub struct EditProfilePage {
    username: TextInput,
    bio: TextInput,
    name_color: TextInput,
    hovered: usize,
    editing: bool,
    update_btn: Button,
    error: Option<String>,
    /// `None` only while a save is in flight.
    me: Option<Me>,
    job: Option<Receiver<(Me, Result<(), String>)>>,
    done: bool,
}

impl EditProfilePage {
    pub fn new(me: Me) -> Self {
        let mut username = TextInput::new();
        username.set_value(&me.user.username);
        let mut bio = TextInput::new();
        bio.set_value(&me.user.bio);
        let mut name_color = TextInput::new();
        name_color.set_value(me.user.name_color.as_deref().unwrap_or(""));

        Self {
            username,
            bio,
            name_color,
            hovered: 0,
            editing: false,
            update_btn: Button::new("Update"),
            error: None,
            me: Some(me),
            job: None,
            done: false,
        }
    }

    fn hovered_item(&self) -> FormItem {
        FORM_ITEMS[self.hovered]
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// Returns true if the whole page should close (Esc while not mid-edit of a field).
    pub fn exit_editing(&mut self) -> bool {
        if self.editing {
            self.editing = false;
            false
        } else {
            true
        }
    }

    pub fn should_close(&self) -> bool {
        self.done
    }

    /// Hands back the (possibly updated) `Me`. Call once while `should_close()` is true, or after a cancel.
    pub fn take_me(&mut self) -> Option<Me> {
        self.me.take()
    }

    pub fn tick(&mut self) {
        self.update_btn.tick();
        self.poll_job();
    }

    fn poll_job(&mut self) {
        let Some(rx) = &self.job else { return };
        match rx.try_recv() {
            Ok((me, Ok(()))) => {
                self.job = None;
                self.update_btn.busy = false;
                self.error = None;
                self.me = Some(me);
                self.done = true;
            }
            Ok((me, Err(msg))) => {
                self.job = None;
                self.update_btn.busy = false;
                self.error = Some(msg);
                self.me = Some(me);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.update_btn.busy = false;
            }
        }
    }

    fn start_update(&mut self) {
        let Some(me) = self.me.take() else { return };
        self.update_btn.busy = true;
        self.error = None;
        let username = self.username.value().to_string();
        let bio = self.bio.value().to_string();
        let name_color_raw = self.name_color.value().trim().to_string();
        let name_color = if name_color_raw.is_empty() {
            None
        } else {
            Some(name_color_raw)
        };
        self.job = Some(spawn_update(me, username, bio, name_color));
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if self.editing {
            if code == KeyCode::Enter || code == KeyCode::Esc {
                self.editing = false;
            } else {
                match self.hovered_item() {
                    FormItem::Username => {
                        self.username.handle_key(code);
                    }
                    FormItem::Bio => {
                        self.bio.handle_key(code);
                    }
                    FormItem::NameColor => {
                        self.name_color.handle_key(code);
                    }
                    FormItem::Update => {}
                }
            }
            return;
        }
        if self.update_btn.busy {
            return;
        }
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.hovered = self.hovered.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.hovered = (self.hovered + 1).min(FORM_ITEMS.len() - 1)
            }
            KeyCode::Enter => {
                if self.hovered_item() == FormItem::Update {
                    self.start_update();
                } else {
                    self.editing = true;
                }
            }
            _ => {}
        }
    }

    pub fn hints(&self) -> Vec<(String, String)> {
        if self.editing {
            vec![
                ("←".into(), "Cursor Left".into()),
                ("→".into(), "Cursor Right".into()),
                ("↵".into(), "Confirm".into()),
                ("Esc".into(), "Stop Editing".into()),
            ]
        } else {
            let edit_or_press = if self.hovered_item() == FormItem::Update {
                "Press"
            } else {
                "Edit"
            };
            vec![
                ("↓/J".into(), "Hover Down".into()),
                ("↑/K".into(), "Hover Up".into()),
                ("↵".into(), edit_or_press.into()),
                ("Esc".into(), "Cancel".into()),
            ]
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Username.
                Constraint::Length(3), // Bio.
                Constraint::Length(3), // Name color.
                Constraint::Length(1), // Spacer.
                Constraint::Length(1), // Error line.
                Constraint::Length(1), // Spacer.
                Constraint::Length(3), // Update button.
            ])
            .split(area);

        let username_hover = self.hovered_item() == FormItem::Username && !self.editing;
        let username_editing = self.hovered_item() == FormItem::Username && self.editing;
        self.username
            .draw(frame, rows[0], "Username", username_hover, username_editing);

        let bio_hover = self.hovered_item() == FormItem::Bio && !self.editing;
        let bio_editing = self.hovered_item() == FormItem::Bio && self.editing;
        self.bio.draw(frame, rows[1], "Bio", bio_hover, bio_editing);

        let color_hover = self.hovered_item() == FormItem::NameColor && !self.editing;
        let color_editing = self.hovered_item() == FormItem::NameColor && self.editing;
        self.name_color.draw(
            frame,
            rows[2],
            "Name Color (hex, optional)",
            color_hover,
            color_editing,
        );

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(format!("Couldn't save: {err}"))
                    .style(Style::default().fg(Color::Red)),
                rows[4],
            );
        }

        self.update_btn
            .draw(frame, rows[6], self.hovered_item() == FormItem::Update);
    }
}
