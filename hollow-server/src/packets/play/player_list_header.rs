// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct PacketPlayerListHeader {
    pub header: Component,
    pub footer: Component,
}

impl Default for PacketPlayerListHeader {
    fn default() -> Self {
        PacketPlayerListHeader {
            header: Component::empty(),
            footer: Component::empty(),
        }
    }
}

impl PacketOut for PacketPlayerListHeader {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        hollow_text::write_component(buf, &self.header, version);
        hollow_text::write_component(buf, &self.footer, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::PlayerListHeader
    }
}
