// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketLoginPluginRequest {
    pub message_id: i32,
    pub channel: String,
    pub data: Vec<u8>,
}

impl PacketOut for PacketLoginPluginRequest {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_var_int(self.message_id);
        buf.write_string(&self.channel);
        buf.write_bytes(&self.data);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::LoginPluginRequest
    }
}
