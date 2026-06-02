// SPDX-License-Identifier: GPL-3.0-only

//! Port of CmdStop.

use std::sync::Arc;

use crate::server::LimboServer;

pub fn execute(server: &Arc<LimboServer>) {
    server.request_shutdown();
}
