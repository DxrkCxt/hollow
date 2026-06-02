// SPDX-License-Identifier: GPL-3.0-only

use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketLoginSuccess {
    pub uuid: Uuid,
    pub username: String,
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
    }

    fn kind(&self) -> PacketKind {
        PacketKind::LoginSuccess
    }
}
