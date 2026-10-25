/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use lyntr_rs::LyntrClient;
use std::sync::RwLock;

struct ClientState {
    client: LyntrClient,
    my_id: String,     // `Me.user.id`, the signed-in account.
    client_id: String, // The API client ID (`Cred.id`) used to sign in.
}

static STATE: RwLock<Option<ClientState>> = RwLock::new(None);

pub fn set(client: LyntrClient, my_id: String, client_id: String) {
    *STATE.write().unwrap() = Some(ClientState {
        client,
        my_id,
        client_id,
    });
}

pub fn clear() {
    *STATE.write().unwrap() = None;
}

pub fn get() -> LyntrClient {
    STATE
        .read()
        .unwrap()
        .as_ref()
        .expect("LyntrClient requested before sign-in")
        .client
        .clone()
}

pub fn my_id() -> String {
    STATE
        .read()
        .unwrap()
        .as_ref()
        .expect("my_id requested before sign-in")
        .my_id
        .clone()
}

pub fn current_client_id() -> String {
    STATE
        .read()
        .unwrap()
        .as_ref()
        .expect("current_client_id requested before sign-in")
        .client_id
        .clone()
}
