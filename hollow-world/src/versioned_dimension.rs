// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crab_nbt::NbtCompound;

use hollow_protocol::registry::Version;
use hollow_data::NamespacedKey;
use crate::dimension::Dimension;

#[derive(Clone, Debug)]
pub struct VersionedDimension {
    pub key: NamespacedKey,
    pub legacy_dimension_id: i32,
    pub per_version_dimensions: HashMap<Version, Dimension>,
}

impl VersionedDimension {
    pub fn new(
        key: NamespacedKey,
        legacy_dimension_id: i32,
        per_version_dimensions: HashMap<Version, Dimension>,
    ) -> VersionedDimension {
        VersionedDimension {
            key,
            legacy_dimension_id,
            per_version_dimensions,
        }
    }

    pub fn key(&self) -> &NamespacedKey {
        &self.key
    }

    pub fn legacy_dimension_id(&self) -> i32 {
        self.legacy_dimension_id
    }

    fn by_protocol(&self, version: Version) -> &Dimension {
        self.per_version_dimensions
            .get(&version)
            .unwrap_or_else(|| panic!("No dimension found for version {version:?}"))
    }

    pub fn id(&self, version: Version) -> i32 {
        self.by_protocol(version).id
    }

    pub fn height(&self, version: Version) -> i32 {
        self.by_protocol(version).height
    }

    pub fn chunk_sections(&self, version: Version) -> i32 {
        self.height(version) / 16
    }

    pub fn codec(&self, version: Version) -> &NbtCompound {
        &self.by_protocol(version).codec
    }

    pub fn default_codec(&self, version: Version) -> &NbtCompound {
        &self.by_protocol(version).default_codec
    }
}
