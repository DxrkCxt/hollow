// SPDX-License-Identifier: GPL-3.0-only

//! Port of CmdMem — reports process memory (RSS) and system memory.

use sysinfo::{ProcessesToUpdate, System, get_current_pid};

use crate::server::log;

pub fn execute() {
    let mut sys = System::new();
    sys.refresh_memory();
    let total = sys.total_memory();
    let used = sys.used_memory();

    let rss = match get_current_pid() {
        Ok(pid) => {
            sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
            sys.process(pid).map(|p| p.memory()).unwrap_or(0)
        }
        Err(_) => 0,
    };

    let mb = |bytes: u64| bytes / (1024 * 1024);
    log::info(format!("Memory usage (process RSS): {} MB", mb(rss)));
    log::info(format!(
        "System memory: {} / {} MB used",
        mb(used),
        mb(total)
    ));
}
