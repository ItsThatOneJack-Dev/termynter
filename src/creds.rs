/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Cred {
    pub secret: String,
    pub id: String,
}

fn path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("termynter")
        .join("creds.json")
}

pub fn load() -> Vec<Cred> {
    let Ok(data) = fs::read_to_string(path()) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(creds: &[Cred]) -> io::Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(creds).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// Moves (or inserts) `cred` to the front (the "last used" slot), and persists it.
pub fn promote(cred: Cred) {
    let mut creds = load();
    creds.retain(|c| c.id != cred.id);
    creds.insert(0, cred);
    let _ = save(&creds);
}

pub fn remove(client_id: &str) {
    let mut creds = load();
    creds.retain(|c| c.id != client_id);
    let _ = save(&creds);
}
