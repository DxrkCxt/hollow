// SPDX-License-Identifier: GPL-3.0-only

pub const VELOCITY_INFO_CHANNEL: &str = "velocity:player_info";
pub const BRAND_CHANNEL: &str = "minecraft:brand";

/// Limbo version, equivalent to BuildConfig.LIMBO_VERSION (from Cargo package version).
pub const LIMBO_VERSION: &str = env!("CARGO_PKG_VERSION");
