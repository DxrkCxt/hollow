// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct PacketTitleSetSubTitle {
    pub subtitle: Component,
}

impl Default for PacketTitleSetSubTitle {
    fn default() -> Self {
        PacketTitleSetSubTitle { subtitle: Component::empty() }
    }
}

impl PacketOut for PacketTitleSetSubTitle {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        hollow_text::write_component(buf, &self.subtitle, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::TitleSetSubTitle
    }
}
