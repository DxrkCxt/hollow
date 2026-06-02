// SPDX-License-Identifier: GPL-3.0-only

//! Port of CmdConn.

use std::sync::Arc;

use crate::server::LimboServer;
use crate::server::log;

pub fn execute(server: &Arc<LimboServer>) {
    log::info(format!("Connections: {}", server.connections.count()));
}
