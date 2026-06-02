// SPDX-License-Identifier: GPL-3.0-only

//! UUID helpers.

use md5::{Digest, Md5};
use uuid::Uuid;

/// Offline-mode UUID: `UUID.nameUUIDFromBytes(("OfflinePlayer:" + name).getBytes(UTF_8))`.
/// This is an MD5 (version 3) name-based UUID without a namespace, so it cannot use
/// `Uuid::new_v3` (which prepends a namespace) — we reproduce nameUUIDFromBytes directly.
pub fn offline_mode_uuid(username: &str) -> Uuid {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes: [u8; 16] = hasher.finalize().into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // IETF variant
    Uuid::from_bytes(bytes)
}

/// Parse a dashed or undashed (Mojang) UUID string, port of UUIDUtils.fromString.
pub fn from_string(s: &str) -> Option<Uuid> {
    if s.contains('-') {
        return Uuid::parse_str(s).ok();
    }
    if s.len() != 32 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let dashed = format!(
        "{}-{}-{}-{}-{}",
        &s[0..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    );
    Uuid::parse_str(&dashed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_known_value() {
        // Matches Bukkit/Java UUID.nameUUIDFromBytes("OfflinePlayer:Notch").
        let u = offline_mode_uuid("Notch");
        assert_eq!(u.to_string(), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn offline_uuid_is_version_3() {
        let u = offline_mode_uuid("Hollow");
        assert_eq!(u.get_version_num(), 3);
    }

    #[test]
    fn from_string_dashed_and_undashed() {
        let dashed = "b50ad385-829d-3141-a216-7e7d7539ba7f";
        let undashed = "b50ad385829d3141a2167e7d7539ba7f";
        assert_eq!(from_string(dashed), Uuid::parse_str(dashed).ok());
        assert_eq!(from_string(undashed).map(|u| u.to_string()).as_deref(), Some(dashed));
    }
}
