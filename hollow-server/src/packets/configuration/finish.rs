// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

/// Empty outbound finish-configuration packet (no body).
#[derive(Clone, Debug, Default)]
pub struct PacketFinishConfiguration;

impl PacketOut for PacketFinishConfiguration {
    fn encode(&self, _buf: &mut ByteMessage, _version: Version) {
        // No body.
    }

    fn kind(&self) -> PacketKind {
        PacketKind::FinishConfiguration
    }
}
