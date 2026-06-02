// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct PacketDisconnect {
    pub reason: Component,
}

impl Default for PacketDisconnect {
    fn default() -> Self {
        PacketDisconnect { reason: Component::empty() }
    }
}

impl PacketOut for PacketDisconnect {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        hollow_text::write_component(buf, &self.reason, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::Disconnect
    }
}
