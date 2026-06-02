// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_data::NamespacedKey;

#[derive(Clone, Debug)]
pub struct PacketSpawnPosition {
    pub dimension_key: NamespacedKey,
    pub x: i64,
    pub y: i64,
    pub z: i64,
    pub yaw: f32,
    pub pitch: f32,
}

impl PacketOut for PacketSpawnPosition {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        if version.more_or_equal(Version::V1_21_9) {
            buf.write_namespaced_key(&self.dimension_key.to_string());
        }
        buf.write_i64(encode_position(self.x, self.y, self.z));
        buf.write_f32(self.yaw);
        if version.more_or_equal(Version::V1_21_9) {
            buf.write_f32(self.pitch);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::SpawnPosition
    }
}

impl Default for PacketSpawnPosition {
    fn default() -> Self {
        PacketSpawnPosition {
            dimension_key: NamespacedKey::minecraft("overworld"),
            x: 0,
            y: 0,
            z: 0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn encode_position(x: i64, y: i64, z: i64) -> i64 {
    ((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)
}
