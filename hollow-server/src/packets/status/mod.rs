// SPDX-License-Identifier: GPL-3.0-only

//! Status-state outbound packet structs.

pub mod ping;
pub mod response;

pub use ping::PacketStatusPing;
pub use response::{PacketStatusResponse, PlayerInfo};
