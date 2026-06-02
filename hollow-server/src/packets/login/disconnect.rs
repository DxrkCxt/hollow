// SPDX-License-Identifier: GPL-3.0-only

//!
//! The Java original always writes the component as a JSON string (even on 1.20.3+)
//! via `gsonComponentSerializer.serialize(reason)`.  The login state does not use
//! the NBT-encoded component wire format, so we serialize to JSON and write it as
//! a string regardless of version.

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::{self, Component};

#[derive(Clone, Debug)]
pub struct PacketLoginDisconnect {
    pub reason: Component,
}

impl Default for PacketLoginDisconnect {
    fn default() -> Self {
        PacketLoginDisconnect { reason: Component::empty() }
    }
}

impl PacketOut for PacketLoginDisconnect {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        let profile = hollow_text::gson::profile_for(version);
        let json = hollow_text::gson::serialize(&self.reason, profile);
        buf.write_string(&json);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::LoginDisconnect
    }
}
