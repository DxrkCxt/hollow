// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct PacketTitleSetTitle {
    pub title: Component,
}

impl Default for PacketTitleSetTitle {
    fn default() -> Self {
        PacketTitleSetTitle { title: Component::empty() }
    }
}

impl PacketOut for PacketTitleSetTitle {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        hollow_text::write_component(buf, &self.title, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::TitleSetTitle
    }
}
