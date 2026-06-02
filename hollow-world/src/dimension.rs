// SPDX-License-Identifier: GPL-3.0-only

use crab_nbt::NbtCompound;

use hollow_data::NamespacedKey;

#[derive(Clone, Debug)]
pub struct Dimension {
    pub key: NamespacedKey,
    pub id: i32,
    pub height: i32,
    pub codec: NbtCompound,
    pub default_codec: NbtCompound,
}
