// SPDX-License-Identifier: GPL-3.0-only

//! Port of CmdVersion.

use crate::constants;
use crate::server::log;

pub fn execute() {
    log::info(format!("Server version: {}", constants::LIMBO_VERSION));
}
