// SPDX-License-Identifier: GPL-3.0-only

//! Plugin Message packet (outbound encode).

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketPluginMessage {
    pub channel: String,
    pub data: Vec<u8>,
}

impl PacketOut for PacketPluginMessage {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_string(&self.channel);
        buf.write_bytes(&self.data);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::PluginMessage
    }
}
