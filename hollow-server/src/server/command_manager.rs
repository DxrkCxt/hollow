// SPDX-License-Identifier: GPL-3.0-only

//! Reads stdin lines on a thread
//! and dispatches console commands.

use std::io::BufRead;
use std::sync::Arc;

use crate::server::LimboServer;
use crate::server::commands;

pub fn start(server: Arc<LimboServer>) {
    let _ = std::thread::Builder::new()
        .name("hollow-commands".to_string())
        .spawn(move || {
            let stdin = std::io::stdin();
            let mut lines = stdin.lock().lines();
            while let Some(Ok(line)) = lines.next() {
                let cmd = line.trim();
                if !cmd.is_empty() {
                    commands::dispatch(&server, cmd);
                }
            }
        });
}
