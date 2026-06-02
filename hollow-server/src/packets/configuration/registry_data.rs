// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::MetadataWriter;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

pub struct PacketRegistryData {
    pub metadata_writer: MetadataWriter,
}

impl PacketOut for PacketRegistryData {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        (self.metadata_writer)(buf, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::RegistryData
    }
}
