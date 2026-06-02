// SPDX-License-Identifier: GPL-3.0-only

//! Leveled console logging.
//!
//! Levels: 0 = errors, 1 = +warnings, 2 = +info, 3 = +debug.

use std::sync::atomic::{AtomicI32, Ordering};

use time::OffsetDateTime;
use time::macros::format_description;

static LEVEL: AtomicI32 = AtomicI32::new(2);

pub fn set_level(level: i32) {
    LEVEL.store(level, Ordering::Relaxed);
}

pub fn level() -> i32 {
    LEVEL.load(Ordering::Relaxed)
}

pub fn is_debug() -> bool {
    level() >= 3
}

fn timestamp() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let fmt = format_description!("[hour]:[minute]:[second]");
    now.format(&fmt).unwrap_or_default()
}

fn emit(tag: &str, message: &str, to_stderr: bool) {
    let line = format!("{} {}: {}", timestamp(), tag, message);
    if to_stderr {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
}

pub fn error(message: impl AsRef<str>) {
    if level() >= 0 {
        emit("ERROR", message.as_ref(), true);
    }
}

pub fn warning(message: impl AsRef<str>) {
    if level() >= 1 {
        emit("WARN", message.as_ref(), false);
    }
}

pub fn info(message: impl AsRef<str>) {
    if level() >= 2 {
        emit("INFO", message.as_ref(), false);
    }
}

pub fn debug(message: impl AsRef<str>) {
    if level() >= 3 {
        emit("DEBUG", message.as_ref(), false);
    }
}
