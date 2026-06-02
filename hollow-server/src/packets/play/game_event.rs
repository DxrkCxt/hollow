// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketGameEvent {
    pub event_type: u8,
    pub value: f32,
}

impl PacketOut for PacketGameEvent {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_u8(self.event_type);
        buf.write_f32(self.value);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::GameEvent
    }
}
