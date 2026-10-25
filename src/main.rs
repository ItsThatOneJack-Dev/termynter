/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

mod account_switcher;
mod client;
mod compose;
mod creds;
mod edit_profile;
mod feed;
mod login;
mod lynt_detail;
mod main_page;
mod my_profile_pane;
mod other_profile_page;
mod search;
mod settings;
mod widgets;

use ratatui::Frame;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, ModifierKeyCode,
};
use ratatui::layout::{Constraint, Direction, Layout};
use std::time::Duration;

use account_switcher::AccountSwitcherPage;
use main_page::MainPage;
use widgets::hint_bar;

enum Screen {
    AccountSwitcher(AccountSwitcherPage),
    Main(MainPage),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut screen = Screen::AccountSwitcher(AccountSwitcherPage::new());
    let mut shift_held = false;

    ratatui::run(move |terminal| -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let mut transition = None;
            let mut should_show_switcher = false;
            match &mut screen {
                Screen::AccountSwitcher(switcher) => {
                    switcher.tick();
                    transition = switcher.selected.take();
                }
                Screen::Main(main) => {
                    main.tick();
                    should_show_switcher = main.signed_out || main.switch_account_requested;
                }
            }
            if let Some(me) = transition {
                screen = Screen::Main(MainPage::new(me));
            } else if should_show_switcher {
                screen = Screen::AccountSwitcher(AccountSwitcherPage::new());
            }

            terminal.draw(|frame| draw(frame, &screen, shift_held))?;

            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Modifier(ModifierKeyCode::LeftShift)
                        | KeyCode::Modifier(ModifierKeyCode::RightShift) => {
                            shift_held =
                                matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if key.kind == KeyEventKind::Press {
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            if key.kind == KeyEventKind::Press {
                                match &mut screen {
                                    Screen::AccountSwitcher(switcher) => switcher.exit_editing(),
                                    Screen::Main(main) => main.exit_editing(),
                                }
                            }
                        }
                        _ => {
                            let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                            match &mut screen {
                                Screen::AccountSwitcher(switcher) => {
                                    switcher.handle_key(key.code, key.kind)
                                }
                                Screen::Main(main) => main.handle_key(key.code, key.kind, shift),
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(())
}

fn draw(frame: &mut Frame, screen: &Screen, shift_held: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    match screen {
        Screen::AccountSwitcher(switcher) => switcher.draw(frame, rows[0]),
        Screen::Main(main) => main.draw(frame, rows[0]),
    }

    let (hints, shift_prefix) = match screen {
        Screen::AccountSwitcher(switcher) => switcher.hints(),
        Screen::Main(main) => main.hints(shift_held),
    };
    hint_bar::draw(frame, rows[1], &hints, shift_prefix);
}
