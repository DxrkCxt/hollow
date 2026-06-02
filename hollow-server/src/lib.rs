// SPDX-License-Identifier: GPL-3.0-only

//! Hollow server runtime: protocol packets, the per-connection read/write pipeline,
//! the accept loop and keep-alive ticker, configuration loading, console commands, and
//! player-info forwarding (legacy / BungeeGuard / Velocity modern).

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::large_enum_variant)]

pub mod configuration;
pub mod connection;
pub mod constants;
pub mod forwarding;
pub mod packets;
pub mod server;

pub use connection::{ClientConnection, ConnShared, GameProfile, PacketSnapshots};
pub use server::{Connections, LimboServer};
