// SPDX-License-Identifier: GPL-3.0-only

//! Hollow — a lightweight Minecraft *limbo* server written in Rust.
//!
//! This is the umbrella crate: it re-exports the workspace crates under short paths so
//! downstream users (and the bundled examples) can depend on a single `hollow` crate.

pub use hollow_data as data;
pub use hollow_protocol as protocol;
pub use hollow_text as text;
pub use hollow_world as world;

pub use hollow_server::{configuration, connection, constants, forwarding, packets, server};
