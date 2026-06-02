// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use hollow_protocol::registry::Version;
use hollow_data::NamespacedKey;
use crate::dimension_registry::DimensionRegistry;
use crate::versioned_dimension::VersionedDimension;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionType {
    Overworld,
    TheEnd,
    TheNether,
}

impl DimensionType {
    pub fn key(self) -> NamespacedKey {
        match self {
            DimensionType::Overworld => NamespacedKey::minecraft("overworld"),
            DimensionType::TheEnd => NamespacedKey::minecraft("the_end"),
            DimensionType::TheNether => NamespacedKey::minecraft("the_nether"),
        }
    }

    pub fn legacy_dimension_id(self) -> i32 {
        match self {
            DimensionType::Overworld => 0,
            DimensionType::TheEnd => 1,
            DimensionType::TheNether => -1,
        }
    }

    pub fn from_name(name: &str) -> Option<DimensionType> {
        Some(match name.to_ascii_uppercase().as_str() {
            "OVERWORLD" => DimensionType::Overworld,
            "THE_END" => DimensionType::TheEnd,
            "THE_NETHER" => DimensionType::TheNether,
            _ => return None,
        })
    }

    pub fn create_versioned_dimension(self, registry: &DimensionRegistry) -> VersionedDimension {
        let key = self.key();
        let mut per_version = HashMap::new();
        for &version in Version::VALUES {
            if let Some(dimension) = registry.find_dimension(version, &key) {
                per_version.insert(version, dimension);
            }
        }
        VersionedDimension::new(key, self.legacy_dimension_id(), per_version)
    }
}
