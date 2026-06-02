// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

const FLAG_INVINCIBLE: u8 = 0x01;
const FLAG_FLYING: u8 = 0x02;
const FLAG_CAN_FLY: u8 = 0x04;
const FLAG_CREATIVE: u8 = 0x08;

#[derive(Clone, Debug, Default)]
pub struct PacketPlayerAbilities {
    pub invincible: bool,
    pub can_fly: bool,
    pub flying: bool,
    pub creative: bool,
    pub flying_speed: f32,
    pub field_of_view: f32,
}

impl PacketOut for PacketPlayerAbilities {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        let mut flags: u8 = 0;
        if self.invincible { flags |= FLAG_INVINCIBLE; }
        if self.can_fly    { flags |= FLAG_CAN_FLY; }
        if self.flying     { flags |= FLAG_FLYING; }
        if self.creative   { flags |= FLAG_CREATIVE; }

        buf.write_u8(flags);
        buf.write_f32(self.flying_speed);
        buf.write_f32(self.field_of_view);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::PlayerAbilities
    }
}
