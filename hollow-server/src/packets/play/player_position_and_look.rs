// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketPlayerPositionAndLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub teleport_id: i32,
}

impl PacketOut for PacketPlayerPositionAndLook {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        if version.more_or_equal(Version::V1_21_2) {
            self.encode_modern(buf);
        } else {
            self.encode_legacy(buf, version);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::PlayerPositionAndLook
    }
}

impl PacketPlayerPositionAndLook {
    fn encode_legacy(&self, buf: &mut ByteMessage, version: Version) {
        buf.write_f64(self.x);
        // Pre-1.8: y includes player eye offset of 1.62
        let y = if version.less(Version::V1_8) {
            self.y + 1.62_f64
        } else {
            self.y
        };
        buf.write_f64(y);
        buf.write_f64(self.z);
        buf.write_f32(self.yaw);
        buf.write_f32(self.pitch);

        if version.more_or_equal(Version::V1_8) {
            buf.write_u8(0x08);
        } else {
            buf.write_bool(true);
        }

        if version.more_or_equal(Version::V1_9) {
            buf.write_var_int(self.teleport_id);
        }

        // 1.17–1.19.3: dismount vehicle boolean
        if version.from_to(Version::V1_17, Version::V1_19_3) {
            buf.write_bool(false);
        }
    }

    fn encode_modern(&self, buf: &mut ByteMessage) {
        buf.write_var_int(self.teleport_id);
        buf.write_f64(self.x);
        buf.write_f64(self.y);
        buf.write_f64(self.z);
        // velocity deltas (all zero)
        buf.write_f64(0.0);
        buf.write_f64(0.0);
        buf.write_f64(0.0);
        buf.write_f32(self.yaw);
        buf.write_f32(self.pitch);
        buf.write_i32(0x08);
    }
}
