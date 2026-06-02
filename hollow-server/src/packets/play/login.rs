// SPDX-License-Identifier: GPL-3.0-only

//! Join Game packet (PacketLogin in the Java sources).

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_world::VersionedDimension;

#[derive(Clone, Debug)]
pub struct PacketLogin {
    pub entity_id: i32,
    pub hardcore: bool,
    pub game_mode: i32,
    pub previous_game_mode: i32,
    pub dimension: VersionedDimension,
    pub level_type: String,
    pub seed: i64,
    pub difficulty: i32,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub debug: bool,
    pub flat: bool,
    pub normal_respawn: bool,
    pub limited_crafting: bool,
    pub portal_cooldown: i32,
    pub sea_level: i32,
    pub secure_profile: bool,
}

impl Default for PacketLogin {
    fn default() -> Self {
        panic!("PacketLogin requires a VersionedDimension; construct explicitly")
    }
}

impl PacketLogin {
    pub fn with_dimension(dimension: VersionedDimension) -> Self {
        PacketLogin {
            entity_id: 0,
            hardcore: false,
            game_mode: 2,
            previous_game_mode: -1,
            dimension,
            level_type: "flat".to_string(),
            seed: 0,
            difficulty: 0,
            max_players: 0,
            view_distance: 2,
            simulation_distance: 2,
            reduced_debug_info: false,
            enable_respawn_screen: false,
            debug: false,
            flat: false,
            normal_respawn: true,
            limited_crafting: false,
            portal_cooldown: 0,
            sea_level: 0,
            secure_profile: false,
        }
    }
}

impl PacketOut for PacketLogin {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        buf.write_i32(self.entity_id);

        if version.more_or_equal(Version::V1_16_2) {
            buf.write_bool(self.hardcore);
        }

        // Game mode (only for pre-1.20.2)
        if version.less(Version::V1_20_2) {
            if version.less_or_equal(Version::V1_7_6) {
                // spectator (3) maps to survival (1) for 1.7
                buf.write_u8(if self.game_mode == 3 { 1 } else { self.game_mode as u8 });
            } else {
                buf.write_u8(self.game_mode as u8);
            }
        }

        // 1.16+: previous game mode and dimension key array
        if version.more_or_equal(Version::V1_16) {
            if version.less(Version::V1_20_2) {
                buf.write_i8(self.previous_game_mode as i8);
            }

            // Dimension key array: [dimension.key]
            buf.write_var_int(1); // array length = 1
            buf.write_namespaced_key(&self.dimension.key().to_string());

            if version.less(Version::V1_20_2) {
                buf.write_compound_tag(self.dimension.codec(version), version);
            }
        }

        // Dimension type / id
        if version.more_or_equal(Version::V1_16) {
            if version.more_or_equal(Version::V1_16_2) && version.less(Version::V1_19) {
                // 1.16.2–1.18.2: dimension compound tag (default codec)
                buf.write_compound_tag(self.dimension.default_codec(version), version);
            } else if version.less(Version::V1_20_2) {
                // 1.19–1.20.1: just the namespaced key
                buf.write_namespaced_key(&self.dimension.key().to_string());
            }
            if version.less(Version::V1_20_2) {
                // Dimension name (world name)
                buf.write_namespaced_key(&self.dimension.key().to_string());
            }
        } else if version.more_or_equal(Version::V1_9) {
            buf.write_i32(self.dimension.legacy_dimension_id());
        } else {
            buf.write_i8(self.dimension.legacy_dimension_id() as i8);
        }

        // Hashed seed (1.15+, pre-1.20.2)
        if version.more_or_equal(Version::V1_15) && version.less(Version::V1_20_2) {
            buf.write_i64(self.seed);
        }

        // Difficulty (pre-1.14)
        if version.less(Version::V1_14) {
            buf.write_u8(self.difficulty as u8);
        }

        // Max players
        if version.more_or_equal(Version::V1_16_2) {
            buf.write_var_int(self.max_players);
        } else {
            buf.write_u8(self.max_players as u8);
        }

        // Level type (pre-1.16)
        if version.less(Version::V1_16) {
            buf.write_string(&self.level_type);
        }

        // View distance (1.14+)
        if version.more_or_equal(Version::V1_14) {
            buf.write_var_int(self.view_distance);
        }

        // Simulation distance (1.18+)
        if version.more_or_equal(Version::V1_18) {
            buf.write_var_int(self.simulation_distance);
        }

        // Reduced debug info (1.8+)
        if version.more_or_equal(Version::V1_8) {
            buf.write_bool(self.reduced_debug_info);
        }

        // Enable respawn screen / normal respawn (1.15+)
        if version.more_or_equal(Version::V1_15) {
            buf.write_bool(self.normal_respawn);
        }

        // 1.20.2+ block
        if version.more_or_equal(Version::V1_20_2) {
            buf.write_bool(self.limited_crafting);
            if version.more_or_equal(Version::V1_20_5) {
                buf.write_var_int(self.dimension.id(version));
            } else {
                buf.write_namespaced_key(&self.dimension.key().to_string());
            }
            // Dimension name (world name)
            buf.write_namespaced_key(&self.dimension.key().to_string());
            buf.write_i64(self.seed);
            buf.write_u8(self.game_mode as u8);
            buf.write_i8(self.previous_game_mode as i8);
        }

        // Debug / flat (1.16+)
        if version.more_or_equal(Version::V1_16) {
            buf.write_bool(self.debug);
            buf.write_bool(self.flat);
        }

        // Death location (1.19+): always false
        if version.more_or_equal(Version::V1_19) {
            buf.write_bool(false);
        }

        // Portal cooldown (1.20+)
        if version.more_or_equal(Version::V1_20) {
            buf.write_var_int(self.portal_cooldown);
        }

        // Sea level (1.21.2+)
        if version.more_or_equal(Version::V1_21_2) {
            buf.write_var_int(self.sea_level);
        }

        // Secure profile (1.20.5+)
        if version.more_or_equal(Version::V1_20_5) {
            buf.write_bool(self.secure_profile);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::JoinGame
    }
}
