// SPDX-License-Identifier: GPL-3.0-only

use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::Component;

/// Mirrors Java's PacketChatMessage.PositionLegacy enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PositionLegacy {
    #[default]
    Chat,
    SystemMessage,
    ActionBar,
}

impl PositionLegacy {
    pub fn index(self) -> i32 {
        match self {
            PositionLegacy::Chat => 0,
            PositionLegacy::SystemMessage => 1,
            PositionLegacy::ActionBar => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PacketChatMessage {
    pub message: Component,
    pub position: PositionLegacy,
    pub sender: Uuid,
}

impl Default for PacketChatMessage {
    fn default() -> Self {
        PacketChatMessage {
            message: Component::empty(),
            position: PositionLegacy::Chat,
            sender: Uuid::nil(),
        }
    }
}

impl PacketOut for PacketChatMessage {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        hollow_text::write_component(buf, &self.message, version);

        if version.more_or_equal(Version::V1_19_1) {
            // Write boolean: true if ACTION_BAR
            buf.write_bool(self.position == PositionLegacy::ActionBar);
        } else if version.more_or_equal(Version::V1_19) {
            buf.write_var_int(self.position.index());
        } else if version.more_or_equal(Version::V1_8) {
            buf.write_u8(self.position.index() as u8);
        }

        if version.more_or_equal(Version::V1_16) && version.less(Version::V1_19) {
            buf.write_uuid(self.sender);
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::ChatMessage
    }
}
