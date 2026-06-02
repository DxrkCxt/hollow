// SPDX-License-Identifier: GPL-3.0-only

//! Packet contracts shared by the whole protocol layer.
//!
//! Outbound packets implement `PacketOut`. Inbound packets are decoded into the
//! data-carrying `Inbound` enum (so the handler is decoupled from packet structs);
//! `decode_inbound` is implemented in `protocol::packets`.

use uuid::Uuid;

use crate::ByteMessage;
use crate::registry::{PacketKind, Version};

/// An outbound packet. `encode` writes only the packet body (the id is written by
/// the encoder via `kind`).
pub trait PacketOut: Send + Sync {
    fn encode(&self, buf: &mut ByteMessage, version: Version);
    fn kind(&self) -> PacketKind;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakeIntent {
    Status,
    Login,
    Transfer,
    Unknown,
}

impl HandshakeIntent {
    pub fn of(value: i32) -> HandshakeIntent {
        match value {
            1 => HandshakeIntent::Status,
            2 => HandshakeIntent::Login,
            3 => HandshakeIntent::Transfer,
            _ => HandshakeIntent::Unknown,
        }
    }
}

/// Decoded inbound packets, carrying only what the handler needs.
#[derive(Clone, Debug)]
pub enum Inbound {
    Handshake {
        version: Version,
        host: String,
        port: u16,
        intent: HandshakeIntent,
    },
    StatusRequest,
    StatusPing {
        payload: i64,
    },
    LoginStart {
        username: String,
        uuid: Option<Uuid>,
    },
    LoginPluginResponse {
        message_id: i32,
        success: bool,
        data: Option<Vec<u8>>,
    },
    LoginAcknowledged,
    FinishConfiguration,
    KnownPacks,
    /// Inbound keep-alive (configuration or play); content is ignored by the handler.
    KeepAlive {
        id: i64,
    },
    /// Inbound plugin message (configuration); ignored by the handler.
    PluginMessage {
        channel: String,
        data: Vec<u8>,
    },
}
