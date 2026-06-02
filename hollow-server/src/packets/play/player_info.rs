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
            // => 0b00001101 = 0x0D, padded to (6+8)>>3 = 1 byte
            buf.write_fixed_bit_set(&[true, false, true, true, false, false], 6);

            buf.write_var_int(1); // array length
            buf.write_uuid(self.uuid);
            buf.write_string(&self.username);
            buf.write_var_int(0); // properties: empty

            buf.write_bool(true);           // UPDATE_LISTED: listed = true
            buf.write_var_int(self.game_mode); // UPDATE_GAMEMODE: game mode
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
