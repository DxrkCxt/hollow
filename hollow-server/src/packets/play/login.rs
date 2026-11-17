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
    /// 26.2+. Vanilla sends `MinecraftServer.usesAuthentication()`; a limbo (and any
    /// backend behind a proxy) runs in offline mode, so this stays `false`.
    pub online_mode: bool,
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
            online_mode: false,
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
            if version.more_or_equal(Version::V26_3) {
                // 26.3: GameType.STREAM_CODEC (VarInt id) and GameType.OPTIONAL_STREAM_CODEC
                // (ByteBufCodecs.OPTIONAL_VAR_INT: 0 = none, otherwise id + 1).
                buf.write_var_int(self.game_mode);
                buf.write_var_int(if self.previous_game_mode < 0 {
                    0
                } else {
                    self.previous_game_mode + 1
                });
            } else {
                buf.write_u8(self.game_mode as u8);
                buf.write_i8(self.previous_game_mode as i8);
            }
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

        // Online mode (26.2+), between the spawn info and enforcesSecureChat
        if version.more_or_equal(Version::V26_2) {
            buf.write_bool(self.online_mode);
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

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_world::{DimensionRegistry, DimensionType};

    fn encode(version: Version, previous_game_mode: i32) -> Vec<u8> {
        let registry = DimensionRegistry::load();
        let mut packet = PacketLogin::with_dimension(
            DimensionType::Overworld.create_versioned_dimension(&registry),
        );
        packet.game_mode = 3;
        packet.previous_game_mode = previous_game_mode;
        packet.secure_profile = true;
        let mut buf = ByteMessage::new();
        packet.encode(&mut buf, version);
        buf.to_byte_array()
    }

    /// Bytes after the seed: game mode, previous game mode, debug, flat, no death location,
    /// portal cooldown, sea level, [online mode on 26.2+], enforces secure chat.
    fn tail(bytes: &[u8], len: usize) -> &[u8] {
        &bytes[bytes.len() - len..]
    }

    #[test]
    fn online_mode_added_in_26_2() {
        let v26_1 = encode(Version::V26_1, -1);
        let v26_2 = encode(Version::V26_2, -1);
        assert_eq!(v26_2.len(), v26_1.len() + 1);
        assert_eq!(tail(&v26_1, 8), &[3, 0xFF, 0, 0, 0, 0, 0, 1]);
        assert_eq!(tail(&v26_2, 9), &[3, 0xFF, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(&v26_1[..v26_1.len() - 1], &v26_2[..v26_1.len() - 1]);
    }

    #[test]
    fn game_modes_are_var_ints_in_26_3() {
        // No previous game mode -> OPTIONAL_VAR_INT 0 (not the byte 0xFF used before 26.3).
        let none = encode(Version::V26_3, -1);
        assert_eq!(tail(&none, 9), &[3, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(none.len(), encode(Version::V26_2, -1).len());

        // Previous game mode n is written as n + 1.
        let creative = encode(Version::V26_3, 1);
        assert_eq!(tail(&creative, 9), &[3, 2, 0, 0, 0, 0, 0, 0, 1]);
    }
}
