/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::{LyntrClient, Me};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, List, ListItem, ListState};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::creds::{self, Cred};
use crate::login::LoginPage;

enum EntryState {
    Checking,
    Valid { client: LyntrClient, me: Me },
    Invalid,
}

struct Entry {
    cred: Cred,
    state: EntryState,
}

fn spawn_check(cred: Cred) -> Receiver<(Cred, Result<(LyntrClient, Me), ()>)> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(async {
            let client = LyntrClient::new(&cred.id, &cred.secret).await.ok()?;
            let me = client.me().await.ok()?;
            Some((client, me))
        });
        let _ = tx.send((cred, result.ok_or(())));
    });
    rx
}

enum Mode {
    List,
    AddAccount(LoginPage),
}

pub struct AccountSwitcherPage {
    entries: Vec<Entry>,
    jobs: Vec<Receiver<(Cred, Result<(LyntrClient, Me), ()>)>>,
    hovered: usize,
    mode: Mode,
    pub selected: Option<Me>,
    auto_login_pending: bool,
}

impl AccountSwitcherPage {
    pub fn new() -> Self {
        let creds = creds::load();
        let mut entries = Vec::new();
        let mut jobs = Vec::new();
        for cred in creds {
            jobs.push(spawn_check(cred.clone()));
            entries.push(Entry {
                cred,
                state: EntryState::Checking,
            });
        }
        let auto_login_pending = !entries.is_empty();
        Self {
            entries,
            jobs,
            hovered: 0,
            mode: Mode::List,
            selected: None,
            auto_login_pending,
        }
    }

    pub fn exit_editing(&mut self) {
        if let Mode::AddAccount(login) = &mut self.mode {
            login.exit_editing();
        }
    }

    pub fn tick(&mut self) {
        let mut i = 0;
        while i < self.jobs.len() {
            match self.jobs[i].try_recv() {
                Ok((cred, Ok((client, me)))) => {
                    if let Some(e) = self.entries.iter_mut().find(|e| e.cred.id == cred.id) {
                        e.state = EntryState::Valid { client, me };
                    }
                    self.jobs.remove(i);
                }
                Ok((cred, Err(()))) => {
                    if let Some(e) = self.entries.iter_mut().find(|e| e.cred.id == cred.id) {
                        e.state = EntryState::Invalid;
                    }
                    creds::remove(&cred.id);
                    self.jobs.remove(i);
                }
                Err(TryRecvError::Empty) => i += 1,
                Err(TryRecvError::Disconnected) => {
                    self.jobs.remove(i);
                }
            }
        }
        self.entries
            .retain(|e| !matches!(e.state, EntryState::Invalid));

        if self.auto_login_pending {
            match self.entries.first().map(|e| &e.state) {
                Some(EntryState::Valid { .. }) => {
                    self.select(0);
                    self.auto_login_pending = false;
                }
                Some(EntryState::Invalid) | None if self.jobs.is_empty() => {
                    self.auto_login_pending = false
                }
                _ => {}
            }
        }

        if let Mode::AddAccount(login) = &mut self.mode {
            login.tick();
            if let Some(me) = login.completed.take() {
                let cred = Cred {
                    id: login.client_id.value().to_string(),
                    secret: login.client_secret.value().to_string(),
                };
                creds::promote(cred);
                self.selected = Some(me);
            }
        }
    }

    fn select(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        let entry = self.entries.remove(index);
        if let EntryState::Valid { client, me } = entry.state {
            crate::client::set(client, me.user.id.clone(), entry.cred.id.clone());
            creds::promote(entry.cred);
            self.selected = Some(me);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, kind: KeyEventKind) {
        if kind != KeyEventKind::Press {
            return;
        }
        if let Mode::AddAccount(login) = &mut self.mode {
            login.handle_key(code, kind);
            return;
        }
        let total = self.entries.len() + 1;
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.hovered = self.hovered.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.hovered = (self.hovered + 1).min(total - 1)
            }
            KeyCode::Enter => {
                if self.hovered == self.entries.len() {
                    self.mode = Mode::AddAccount(LoginPage::new());
                } else if matches!(
                    self.entries.get(self.hovered).map(|e| &e.state),
                    Some(EntryState::Valid { .. })
                ) {
                    self.select(self.hovered);
                }
            }
            _ => {}
        }
    }

    pub fn hints(&self) -> (Vec<(String, String)>, bool) {
        if let Mode::AddAccount(login) = &self.mode {
            return login.hints(false);
        }
        (
            vec![
                ("↓/J".into(), "Down".into()),
                ("↑/K".into(), "Up".into()),
                ("↵".into(), "Select".into()),
            ],
            false,
        )
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        if let Mode::AddAccount(login) = &self.mode {
            login.draw(frame, area);
            return;
        }

        let block = Block::bordered().title("Switch Account");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| match &e.state {
                EntryState::Checking => ListItem::new("Checking…"),
                EntryState::Valid { me, .. } => {
                    ListItem::new(format!("{} (@{})", me.user.username, me.user.handle))
                }
                EntryState::Invalid => ListItem::new("(invalid)"),
            })
            .collect();
        items.push(ListItem::new("+ Add Account"));

        let mut state = ListState::default();
        state.select(Some(self.hovered));
        let list =
            List::new(items).highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));
        frame.render_stateful_widget(list, inner, &mut state);
    }
}
