// SPDX-License-Identifier: GPL-3.0-only

//! Keep Alive packet (outbound encode only).

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketKeepAlive {
    pub id: i64,
}

impl PacketOut for PacketKeepAlive {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        if version.more_or_equal(Version::V1_12_2) {
            buf.write_i64(self.id);
        } else if version.more_or_equal(Version::V1_8) {
            buf.write_var_int(self.id as i32);
        } else {
            buf.write_i32(self.id as i32);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::KeepAlive
    }
}
