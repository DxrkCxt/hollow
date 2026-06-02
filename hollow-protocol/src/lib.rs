// SPDX-License-Identifier: GPL-3.0-only

//! Hollow wire-protocol primitives: the `ByteMessage` buffer, protocol versions and
//! per-(state, version) packet registry, packet contracts, pre-encoded snapshots, and
//! the NBT/UUID/JSON helpers the wire layer needs.

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::large_enum_variant)]

pub mod byte_message;
pub mod json;
pub mod metadata_writer;
pub mod nbt;
pub mod packet;
pub mod registry;
pub mod snapshot;
pub mod uuid_util;

pub use byte_message::{ByteMessage, DecodeError, DecodeResult};
pub use metadata_writer::MetadataWriter;
pub use packet::{HandshakeIntent, Inbound, PacketOut};
pub use snapshot::PacketSnapshot;
