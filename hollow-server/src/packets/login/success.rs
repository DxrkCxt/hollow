// SPDX-License-Identifier: GPL-3.0-only

use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketLoginSuccess {
    pub uuid: Uuid,
    pub username: String,
    /// Session id sent after the game profile on 26.2+. Vanilla shares one random UUID
    /// between all connected players (`ServerConnectionListener.getSessionId()`) and
    /// regenerates it once the server is empty; it is not a per-player value.
    pub session_id: Uuid,
}

impl PacketOut for PacketLoginSuccess {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        if version.more_or_equal(Version::V1_16) {
            buf.write_uuid(self.uuid);
        } else if version.more_or_equal(Version::V1_7_6) {
            buf.write_string(&self.uuid.to_string());
        } else {
            buf.write_string(&self.uuid.to_string().replace('-', ""));
        }
        buf.write_string(&self.username);
        if version.more_or_equal(Version::V1_19) {
            buf.write_var_int(0);
        }
        if version.from_to(Version::V1_20_5, Version::V1_21) {
            buf.write_bool(true);
        }
        if version.more_or_equal(Version::V26_2) {
            buf.write_uuid(self.session_id);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::LoginSuccess
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(version: Version) -> Vec<u8> {
        let packet = PacketLoginSuccess {
            uuid: Uuid::from_u128(1),
            username: "Steve".to_string(),
            session_id: Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff),
        };
        let mut buf = ByteMessage::new();
        packet.encode(&mut buf, version);
        buf.to_byte_array()
    }

    #[test]
    fn session_id_appended_from_26_2() {
        let v26_1 = encode(Version::V26_1);
        // uuid (16) + "Steve" (1 + 5) + empty properties (1)
        assert_eq!(v26_1.len(), 23);

        for version in [Version::V26_2, Version::V26_3] {
            let bytes = encode(version);
            assert_eq!(bytes.len(), v26_1.len() + 16, "{version:?}");
            assert_eq!(&bytes[..v26_1.len()], &v26_1[..]);
            assert_eq!(
                &bytes[v26_1.len()..],
                &0x0011_2233_4455_6677_8899_aabb_ccdd_eeff_u128.to_be_bytes()
            );
        }
    }
}
