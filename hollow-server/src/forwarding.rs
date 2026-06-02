// SPDX-License-Identifier: GPL-3.0-only

//! Player info forwarding helpers.
//!
//! Decoupled from ClientConnection: these take the relevant config and return the
//! extracted data, which the connection layer then applies.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_data::InfoForwarding;
use hollow_protocol::uuid_util as uuid_util;

type HmacSha256 = Hmac<Sha256>;

pub const VELOCITY_MAX_SUPPORTED_FORWARDING_VERSION: u8 = 1;

/// Verify the Velocity modern-forwarding HMAC-SHA256 signature. The buffer must be
/// positioned at the 32-byte signature; on success the reader is left just past the
/// signature so the caller can read version/address/uuid/username next.
pub fn check_velocity_key_integrity(secret_key: &[u8], buf: &mut ByteMessage) -> bool {
    let signature = match buf.read_bytes(32) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let data = buf.as_read_slice();
    let mut mac = match HmacSha256::new_from_slice(secret_key) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(data);
    // Constant-time comparison (MessageDigest.isEqual equivalent).
    mac.verify_slice(&signature).is_ok()
}

/// Validate a BungeeGuard handshake string, returning the real (address, uuid) on success.
pub fn check_bungee_guard_handshake(
    handshake: &str,
    forwarding: &InfoForwarding,
) -> Option<(String, Uuid)> {
    let split: Vec<&str> = handshake.split('\0').collect();
    if split.len() != 4 {
        return None;
    }

    let socket_address_hostname = split[1].to_string();
    let uuid = uuid_util::from_string(split[2])?;

    let token = extract_bungee_guard_token(split[3]);
    if !forwarding.has_token(token.as_deref()) {
        return None;
    }

    Some((socket_address_hostname, uuid))
}

fn extract_bungee_guard_token(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let array = value.as_array()?;
    for element in array {
        let obj = match element.as_object() {
            Some(o) => o,
            None => continue,
        };
        if obj.get("name").and_then(|n| n.as_str()) == Some("bungeeguard-token")
            && let Some(v) = obj.get("value").and_then(|v| v.as_str())
        {
            return Some(v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_data::ForwardingType;

    #[test]
    fn bungee_guard_valid() {
        let forwarding = InfoForwarding {
            forwarding_type: ForwardingType::BungeeGuard,
            secret_key: Vec::new(),
            tokens: vec!["secret-token".to_string()],
        };
        let handshake = format!(
            "host\u{0}1.2.3.4\u{0}{}\u{0}[{{\"name\":\"bungeeguard-token\",\"value\":\"secret-token\"}}]",
            "069a79f444e94726a5befca90e38aaf5"
        );
        let result = check_bungee_guard_handshake(&handshake, &forwarding);
        assert!(result.is_some());
        let (addr, _uuid) = result.unwrap();
        assert_eq!(addr, "1.2.3.4");
    }

    #[test]
    fn bungee_guard_wrong_token() {
        let forwarding = InfoForwarding {
            forwarding_type: ForwardingType::BungeeGuard,
            secret_key: Vec::new(),
            tokens: vec!["expected".to_string()],
        };
        let handshake = "host\u{0}1.2.3.4\u{0}069a79f444e94726a5befca90e38aaf5\u{0}[{\"name\":\"bungeeguard-token\",\"value\":\"wrong\"}]";
        assert!(check_bungee_guard_handshake(handshake, &forwarding).is_none());
    }

    #[test]
    fn velocity_hmac_roundtrip() {
        let secret = b"my-secret";
        let payload = b"forwarded-player-data";
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(payload);
        let sig = mac.finalize().into_bytes();

        let mut buf = ByteMessage::new();
        buf.write_bytes(&sig);
        buf.write_bytes(payload);
        let bytes = buf.to_byte_array();

        let mut read = ByteMessage::from_bytes(&bytes);
        assert!(check_velocity_key_integrity(secret, &mut read));

        let mut read2 = ByteMessage::from_bytes(&bytes);
        assert!(!check_velocity_key_integrity(b"wrong-secret", &mut read2));
    }
}
