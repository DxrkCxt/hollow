// SPDX-License-Identifier: GPL-3.0-only

//! Console commands.

pub mod conn;
pub mod help;
pub mod mem;
pub mod stop;
pub mod version;

use std::sync::Arc;

use crate::server::LimboServer;
use crate::server::log;

pub fn dispatch(server: &Arc<LimboServer>, input: &str) {
    let name = input.split_whitespace().next().unwrap_or("");
    match name {
        "help" => help::execute(),
        "conn" => conn::execute(server),
        "mem" => mem::execute(),
        "version" => version::execute(),
        "stop" => stop::execute(server),
        other => log::info(format!("Unknown command \"{other}\". Type \"help\" for help")),
    }
}
