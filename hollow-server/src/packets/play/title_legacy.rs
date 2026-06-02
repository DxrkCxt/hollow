// SPDX-License-Identifier: GPL-3.0-only

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use super::title_set_title::PacketTitleSetTitle;
use super::title_set_subtitle::PacketTitleSetSubTitle;
use super::title_times::PacketTitleTimes;

/// Mirrors Java's PacketTitleLegacy.Action enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TitleAction {
    #[default]
    SetTitle,
    SetSubtitle,
    SetTimesAndDisplay,
}

impl TitleAction {
    /// Returns the action id for the given version, matching Java's Action.getId(version).
    /// SET_TIMES_AND_DISPLAY: id=3 (>=1.11), legacyId=2 (<1.11)
    /// SET_TITLE: id=0, SET_SUBTITLE: id=1 (same for all versions)
    pub fn id(self, version: Version) -> i32 {
        match self {
            TitleAction::SetTitle => 0,
            TitleAction::SetSubtitle => 1,
            TitleAction::SetTimesAndDisplay => {
                if version.less(Version::V1_11) { 2 } else { 3 }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PacketTitleLegacy {
    pub action: TitleAction,
    pub title: PacketTitleSetTitle,
    pub subtitle: PacketTitleSetSubTitle,
    pub times: PacketTitleTimes,
}

impl PacketOut for PacketTitleLegacy {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        buf.write_var_int(self.action.id(version));

        match self.action {
            TitleAction::SetTitle => self.title.encode(buf, version),
            TitleAction::SetSubtitle => self.subtitle.encode(buf, version),
            TitleAction::SetTimesAndDisplay => self.times.encode(buf, version),
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::TitleLegacy
    }
}
