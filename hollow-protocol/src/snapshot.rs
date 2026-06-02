// SPDX-License-Identifier: GPL-3.0-only

//! Encodes a packet body per MC
//! version, deduplicating identical encodings (here by exact byte content, which is
//! always correct, vs the Java original keyed on ByteBuf.hashCode()).

use std::collections::HashMap;
use std::sync::Arc;

use crate::ByteMessage;
use crate::packet::PacketOut;
use crate::registry::{PacketKind, Version};

#[derive(Clone)]
pub struct PacketSnapshot {
    kind: PacketKind,
    version_messages: HashMap<Version, Arc<[u8]>>,
    mappings: HashMap<Version, Version>,
}

impl PacketSnapshot {
    pub fn kind(&self) -> PacketKind {
        self.kind
    }

    /// Build a snapshot by encoding `encode_fn` for each version (skipping UNDEFINED).
    pub fn build<F>(kind: PacketKind, versions: &[Version], encode_fn: F) -> PacketSnapshot
    where
        F: Fn(Version, &mut ByteMessage),
    {
        let mut version_messages: HashMap<Version, Arc<[u8]>> = HashMap::new();
        let mut mappings: HashMap<Version, Version> = HashMap::new();
        let mut seen: HashMap<Vec<u8>, Version> = HashMap::new();

        for &version in versions {
            if version == Version::Undefined {
                continue;
            }
            let mut msg = ByteMessage::new();
            encode_fn(version, &mut msg);
            let bytes = msg.to_byte_array();

            if let Some(&hashed) = seen.get(&bytes) {
                mappings.insert(version, hashed);
            } else {
                seen.insert(bytes.clone(), version);
                mappings.insert(version, version);
                version_messages.insert(version, Arc::from(bytes.into_boxed_slice()));
            }
        }

        PacketSnapshot {
            kind,
            version_messages,
            mappings,
        }
    }

    /// Encode a single packet instance across all versions (PacketSnapshot.of(packet)).
    pub fn of(packet: &dyn PacketOut) -> PacketSnapshot {
        Self::build(packet.kind(), Version::VALUES, |v, buf| packet.encode(buf, v))
    }

    /// Encode a single packet instance for one version (PacketSnapshot.of(packet, version)).
    pub fn of_version(packet: &dyn PacketOut, version: Version) -> PacketSnapshot {
        Self::build(packet.kind(), &[version], |v, buf| packet.encode(buf, v))
    }
}

impl PacketOut for PacketSnapshot {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        let mapped = self.mappings.get(&version).copied().unwrap_or(version);
        if let Some(message) = self.version_messages.get(&mapped) {
            buf.write_bytes(message);
        }
    }

    fn kind(&self) -> PacketKind {
        self.kind
    }
}
