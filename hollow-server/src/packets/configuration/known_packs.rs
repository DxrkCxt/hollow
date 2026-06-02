// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, Default)]
pub struct PacketKnownPacks {
    pub known_packs: Vec<KnownPack>,
}

impl PacketOut for PacketKnownPacks {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_var_int(self.known_packs.len() as i32);
        for pack in &self.known_packs {
            buf.write_string(&pack.namespace);
            buf.write_string(&pack.id);
            buf.write_string(&pack.version);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::KnownPacks
    }
}
