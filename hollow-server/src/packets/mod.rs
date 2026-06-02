// SPDX-License-Identifier: GPL-3.0-only

//! All protocol packets, grouped by state, plus inbound decoding.

pub mod configuration;
pub mod inbound;
pub mod login;
pub mod play;
pub mod status;

pub use inbound::decode_inbound;
