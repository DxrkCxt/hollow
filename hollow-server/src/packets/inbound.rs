// SPDX-License-Identifier: GPL-3.0-only

//! Inbound packet decoder: maps `PacketKind` + raw buffer -> `Inbound` enum.
//!
//! Each branch mirrors the `decode(ByteMessage, Version)` method in the
//! corresponding Java inbound packet class.

use hollow_protocol::{ByteMessage, DecodeError, DecodeResult, HandshakeIntent, Inbound};
use hollow_protocol::registry::{PacketKind, Version};

const MAX_SHORT: usize = 32767; // Short.MAX_VALUE, used as plugin-message data limit

/// Decode a single inbound packet body from `buf` given its `kind` and `version`.
///
/// `buf` contains only the packet body (packet-id has already been consumed by the
/// pipeline decoder).
pub fn decode_inbound(
    kind: PacketKind,
    buf: &mut ByteMessage,
    version: Version,
) -> DecodeResult<Inbound> {
    match kind {
        // ------------------------------------------------------------------
        // HANDSHAKE
        // PacketHandshake.decode: varint protocolVersion, string host,
        //   unsigned short port, varint nextState mapped to Intent.
        // ------------------------------------------------------------------
        PacketKind::Handshake => {
            let protocol_number = buf.read_var_int()?;
            let resolved_version = Version::of(protocol_number);
            let host = buf.read_string()?;
            let port = buf.read_u16()?;
            let intent_raw = buf.read_var_int()?;
            let intent = HandshakeIntent::of(intent_raw);
            Ok(Inbound::Handshake {
                version: resolved_version,
                host,
                port,
                intent,
            })
        }

        // ------------------------------------------------------------------
        // STATUS
        // PacketStatusRequest.decode: no body.
        // ------------------------------------------------------------------
        PacketKind::StatusRequest => Ok(Inbound::StatusRequest),

        // PacketStatusPing.decode: long randomId.
        PacketKind::StatusPing => {
            let payload = buf.read_i64()?;
            Ok(Inbound::StatusPing { payload })
        }

        // ------------------------------------------------------------------
        // LOGIN
        // PacketLoginStart.decode:
        //   string username (max 16)
        //   [V1_19..V1_19_1] optional PlayerPublicKey (bool flag + long + array(512) + array(4096))
        //   [V1_19_1+] optional uuid:
        //     - on V1_20_2+: always present
        //     - on V1_19_1..V1_20_1: guarded by a bool flag
        // ------------------------------------------------------------------
        PacketKind::LoginStart => {
            let username = buf.read_string_max(16)?;

            // Consume optional PlayerPublicKey (V1_19 through V1_19_1 only).
            // readPublicKey(): if (buf.readBoolean()) { readLong(); readArray(512); readArray(4096); }
            if version.from_to(Version::V1_19, Version::V1_19_1) {
                let has_key = buf.read_bool()?;
                if has_key {
                    let _expiry = buf.read_i64()?;
                    let _public_key = buf.read_array_limited(512)?;
                    let _signature = buf.read_array_limited(4096)?;
                }
            }

            // Optional UUID field (V1_19_1+).
            let uuid = if version.more_or_equal(Version::V1_19_1) {
                // On V1_20_2+ the UUID is always present (no bool guard).
                if version.more_or_equal(Version::V1_20_2) {
                    Some(buf.read_uuid()?)
                } else {
                    // V1_19_1 through V1_20: bool flag guards the UUID.
                    if buf.read_bool()? {
                        Some(buf.read_uuid()?)
                    } else {
                        None
                    }
                }
            } else {
                None
            };

            Ok(Inbound::LoginStart { username, uuid })
        }

        // PacketLoginPluginResponse.decode:
        //   varint messageId, bool successful
        //   if readable bytes > 0: read remaining bytes (limited to Short.MAX_VALUE)
        PacketKind::LoginPluginResponse => {
            let message_id = buf.read_var_int()?;
            let success = buf.read_bool()?;

            let data = if buf.readable_bytes() > 0 {
                let n = buf.readable_bytes();
                if n > MAX_SHORT {
                    return Err(DecodeError(format!(
                        "Cannot receive plugin-response payload larger than {MAX_SHORT}"
                    )));
                }
                Some(buf.read_bytes(n)?)
            } else {
                None
            };

            Ok(Inbound::LoginPluginResponse {
                message_id,
                success,
                data,
            })
        }

        // PacketLoginAcknowledged: no body.
        PacketKind::LoginAcknowledged => Ok(Inbound::LoginAcknowledged),

        // ------------------------------------------------------------------
        // CONFIGURATION
        // PacketFinishConfiguration: no body.
        // ------------------------------------------------------------------
        PacketKind::FinishConfiguration => Ok(Inbound::FinishConfiguration),

        // PacketKnownPacks.decode:
        //   varint size (limited to 16)
        //   for each: string namespace(256), string id(256), string version(256)
        // The decoded packs are discarded — the handler only cares about the
        // event (we ack with our own known-packs list), so we return the unit variant.
        PacketKind::KnownPacks => {
            let size = buf.read_var_int()?;
            if !(0..=16).contains(&size) {
                return Err(DecodeError(format!(
                    "Cannot receive known packs larger than 16 (got {size})"
                )));
            }
            for _ in 0..size {
                let _namespace = buf.read_string_max(256)?;
                let _id = buf.read_string_max(256)?;
                let _version = buf.read_string_max(256)?;
            }
            Ok(Inbound::KnownPacks)
        }

        // ------------------------------------------------------------------
        // KEEP-ALIVE (configuration and play, serverbound)
        // PacketKeepAlive.decode:
        //   V1_12_2+: long id
        //   V1_8+:    varint id
        //   <V1_8:    int id
        // ------------------------------------------------------------------
        PacketKind::KeepAlive => {
            let id = if version.more_or_equal(Version::V1_12_2) {
                buf.read_i64()?
            } else if version.more_or_equal(Version::V1_8) {
                buf.read_var_int()? as i64
            } else {
                buf.read_i32()? as i64
            };
            Ok(Inbound::KeepAlive { id })
        }

        // ------------------------------------------------------------------
        // PLUGIN MESSAGE (configuration and play, serverbound)
        // PacketPluginMessage.decode:
        //   string channel, remaining bytes as data (limited to Short.MAX_VALUE)
        // ------------------------------------------------------------------
        PacketKind::PluginMessage => {
            let channel = buf.read_string()?;
            let n = buf.readable_bytes();
            if n > MAX_SHORT {
                return Err(DecodeError(format!(
                    "Cannot receive plugin-message payload larger than {MAX_SHORT}"
                )));
            }
            let data = if n > 0 { buf.read_bytes(n)? } else { Vec::new() };
            Ok(Inbound::PluginMessage { channel, data })
        }

        // ------------------------------------------------------------------
        // Everything else is unexpected as an inbound packet.
        // ------------------------------------------------------------------
        _ => Err(DecodeError("unexpected inbound kind".into())),
    }
}
