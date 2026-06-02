// SPDX-License-Identifier: GPL-3.0-only

//! Status Ping packet (outbound only).

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketStatusPing {
    pub payload: i64,
}

impl PacketOut for PacketStatusPing {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_i64(self.payload);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::StatusPing
    }
}
