// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketTitleTimes {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}

impl PacketOut for PacketTitleTimes {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_i32(self.fade_in);
        buf.write_i32(self.stay);
        buf.write_i32(self.fade_out);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::TitleTimes
    }
}
