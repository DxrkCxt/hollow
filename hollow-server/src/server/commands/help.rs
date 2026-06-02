// SPDX-License-Identifier: GPL-3.0-only

//! Port of CmdHelp.

use crate::server::log;

pub fn execute() {
    log::info("Available commands:");
    log::info("  help - Show this help message");
    log::info("  conn - Display number of connections");
    log::info("  mem - Display memory usage stats");
    log::info("  version - Display server version");
    log::info("  stop - Stop the server");
}
