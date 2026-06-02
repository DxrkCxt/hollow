// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::connection::client_connection::{ClientConnection, ConnShared};
use crate::server::log;

const REDACTED_ADDRESS: &str = "<redacted>";

pub struct Connections {
    connections: Mutex<HashMap<Uuid, Arc<ConnShared>>>,
    log_players_ip: bool,
}

impl Connections {
    pub fn new(log_players_ip: bool) -> Connections {
        Connections {
            connections: Mutex::new(HashMap::new()),
            log_players_ip,
        }
    }

    pub fn count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }

    pub fn all_shared(&self) -> Vec<Arc<ConnShared>> {
        self.connections.lock().unwrap().values().cloned().collect()
    }

    pub fn add_connection(&self, connection: &ClientConnection) {
        let shared = connection.shared.clone();
        let uuid = shared.uuid().unwrap_or_else(Uuid::nil);
        self.connections.lock().unwrap().insert(uuid, shared.clone());

        let address = if self.log_players_ip {
            shared.address()
        } else {
            REDACTED_ADDRESS.to_string()
        };
        log::info(format!(
            "Player {} connected ({}) [{:?}]",
            shared.username().unwrap_or_default(),
            address,
            connection.version()
        ));
    }

    pub fn remove_connection(&self, connection: &ClientConnection) {
        if let Some(uuid) = connection.shared.uuid() {
            self.connections.lock().unwrap().remove(&uuid);
        }
        log::info(format!(
            "Player {} disconnected",
            connection.shared.username().unwrap_or_default()
        ));
    }
}
