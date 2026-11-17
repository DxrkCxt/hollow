// SPDX-License-Identifier: GPL-3.0-only

//! Simplified to ADD_PLAYER action only, as in the Java original.

use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug)]
pub struct PacketPlayerInfo {
    pub game_mode: i32,
    pub username: String,
    pub uuid: Uuid,
}

impl Default for PacketPlayerInfo {
    fn default() -> Self {
        PacketPlayerInfo {
            game_mode: 3,
            username: String::new(),
            uuid: Uuid::nil(),
        }
    }
}

impl PacketOut for PacketPlayerInfo {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        if version.less(Version::V1_8) {
            // Pre-1.8: username + online bool + ping short
            buf.write_string(&self.username);
            buf.write_bool(true);
            buf.write_i16(0);
        } else if version.more_or_equal(Version::V1_19_3) {
            // 1.19.3+: EnumSet<Action> as fixed bitset (6 bits: ADD_PLAYER=0, INITIALIZE_CHAT=1,
            // UPDATE_GAMEMODE=2, UPDATE_LISTED=3, UPDATE_LATENCY=4, UPDATE_DISPLAY_NAME=5)
            // We send: ADD_PLAYER(bit 0), UPDATE_GAMEMODE(bit 2), UPDATE_LISTED(bit 3)
            // => 0b00001101 = 0x0D, padded to (6+8)>>3 = 1 byte. 1.21.2 and 1.21.4 added
            // UPDATE_LIST_ORDER and UPDATE_HAT (8 actions); vanilla still writes 1 byte.
            buf.write_fixed_bit_set(&[true, false, true, true, false, false], 6);

            buf.write_var_int(1); // array length
            buf.write_uuid(self.uuid);
            // Per-entry action data follows the action enum order: ADD_PLAYER,
            // then UPDATE_GAMEMODE, then UPDATE_LISTED.
            buf.write_string(&self.username);
            buf.write_var_int(0); // properties: empty

            buf.write_var_int(self.game_mode); // UPDATE_GAMEMODE: game mode
            buf.write_bool(true); // UPDATE_LISTED: listed = true
        } else {
            // 1.8–1.19.2: ADD_PLAYER action
            buf.write_var_int(0); // action: add player
            buf.write_var_int(1); // array length
            buf.write_uuid(self.uuid);
            buf.write_string(&self.username);
            buf.write_var_int(0); // properties: empty
            buf.write_var_int(self.game_mode);
            buf.write_var_int(60); // ping
            buf.write_bool(false); // has display name

            if version.more_or_equal(Version::V1_19) {
                buf.write_bool(false); // has chat session
            }
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::PlayerInfo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_entry_writes_game_mode_before_listed() {
        let packet = PacketPlayerInfo {
            game_mode: 3,
            username: "Steve".to_string(),
            uuid: Uuid::from_u128(1),
        };
        for version in [Version::V1_19_3, Version::V1_21_11, Version::V26_3] {
            let mut buf = ByteMessage::new();
            packet.encode(&mut buf, version);
            let bytes = buf.to_byte_array();

            let mut expected = vec![0b0000_1101, 1]; // ADD_PLAYER | UPDATE_GAME_MODE | UPDATE_LISTED, 1 entry
            expected.extend_from_slice(&1u128.to_be_bytes());
            expected.extend_from_slice(&[5, b'S', b't', b'e', b'v', b'e', 0]); // name, no properties
            expected.extend_from_slice(&[3, 1]); // game mode 3, then listed = true
            assert_eq!(bytes, expected, "{version:?}");
        }
    }
}
