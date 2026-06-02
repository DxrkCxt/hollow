// SPDX-License-Identifier: GPL-3.0-only

//! Boss Bar packet (1.9+).

use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_data::BossBar;

#[derive(Clone, Debug)]
pub struct PacketBossBar {
    pub uuid: Uuid,
    pub boss_bar: BossBar,
    pub flags: u8,
}

impl Default for PacketBossBar {
    fn default() -> Self {
        PacketBossBar {
            uuid: Uuid::nil(),
            boss_bar: BossBar {
                text: hollow_text::Component::empty(),
                health: 1.0,
                color: hollow_data::BossBarColor::White,
                division: hollow_data::BossBarDivision::Solid,
            },
            flags: 0,
        }
    }
}

impl PacketOut for PacketBossBar {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        buf.write_uuid(self.uuid);
        buf.write_var_int(0); // action: CREATE bossbar
        hollow_text::write_component(buf, &self.boss_bar.text, version);
        buf.write_f32(self.boss_bar.health);
        buf.write_var_int(self.boss_bar.color.index());
        buf.write_var_int(self.boss_bar.division.index());
        buf.write_u8(self.flags);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::BossBar
    }
}
