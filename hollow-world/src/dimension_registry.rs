// SPDX-License-Identifier: GPL-3.0-only

//!
//! CONTRACT STUB — the public API below is fixed; the body is implemented to load
//! the embedded `dimension/*.nbt` codecs and reproduce the Java lookup logic.

use std::collections::HashMap;

use crab_nbt::{NbtCompound, NbtTag};

use hollow_protocol::metadata_writer::MetadataWriter;
use hollow_protocol::registry::Version;
use hollow_data::NamespacedKey;
use hollow_protocol::nbt::read_gzip_compound;
use crate::dimension::Dimension;

pub struct DimensionRegistry {
    codec_1_16: NbtCompound,
    codec_1_16_2: NbtCompound,
    codec_1_17: NbtCompound,
    codec_1_18_2: NbtCompound,
    codec_1_19: NbtCompound,
    codec_1_19_1: NbtCompound,
    codec_1_19_4: NbtCompound,
    codec_1_20: NbtCompound,
    codec_1_20_5: NbtCompound,
    codec_1_21: NbtCompound,
    codec_1_21_2: NbtCompound,
    codec_1_21_4: NbtCompound,
    codec_1_21_5: NbtCompound,
    codec_1_21_6: NbtCompound,
    codec_1_21_7: NbtCompound,
    codec_1_21_9: NbtCompound,
    codec_1_21_11: NbtCompound,
    codec_26_1: NbtCompound,
    codec_26_2: NbtCompound,
    codec_26_3: NbtCompound,

    tags_1_20_5: NbtCompound,
    tags_1_21: NbtCompound,
    tags_1_21_2: NbtCompound,
    tags_1_21_4: NbtCompound,
    tags_1_21_5: NbtCompound,
    tags_1_21_6: NbtCompound,
    tags_1_21_7: NbtCompound,
    tags_1_21_9: NbtCompound,
    tags_1_21_11: NbtCompound,
    tags_26_1: NbtCompound,
    tags_26_2: NbtCompound,
    tags_26_3: NbtCompound,
}

impl DimensionRegistry {
    /// Load all embedded codec/tags resources (gzip Java NBT).
    pub fn load() -> DimensionRegistry {
        DimensionRegistry {
            codec_1_16: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_16.nbt"
            )),
            codec_1_16_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_16_2.nbt"
            )),
            codec_1_17: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_17.nbt"
            )),
            codec_1_18_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_18_2.nbt"
            )),
            codec_1_19: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_19.nbt"
            )),
            codec_1_19_1: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_19_1.nbt"
            )),
            codec_1_19_4: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_19_4.nbt"
            )),
            codec_1_20: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_20.nbt"
            )),
            codec_1_20_5: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_20_5.nbt"
            )),
            codec_1_21: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21.nbt"
            )),
            codec_1_21_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_2.nbt"
            )),
            codec_1_21_4: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_4.nbt"
            )),
            codec_1_21_5: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_5.nbt"
            )),
            codec_1_21_6: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_6.nbt"
            )),
            codec_1_21_7: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_7.nbt"
            )),
            codec_1_21_9: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_9.nbt"
            )),
            codec_1_21_11: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_1_21_11.nbt"
            )),
            codec_26_1: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_26_1.nbt"
            )),
            codec_26_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_26_2.nbt"
            )),
            codec_26_3: read_gzip_compound(include_bytes!(
                "../resources/dimension/codec_26_3.nbt"
            )),

            tags_1_20_5: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_20_5.nbt"
            )),
            tags_1_21: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21.nbt"
            )),
            tags_1_21_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_2.nbt"
            )),
            tags_1_21_4: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_4.nbt"
            )),
            tags_1_21_5: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_5.nbt"
            )),
            tags_1_21_6: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_6.nbt"
            )),
            tags_1_21_7: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_7.nbt"
            )),
            tags_1_21_9: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_9.nbt"
            )),
            tags_1_21_11: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_1_21_11.nbt"
            )),
            tags_26_1: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_26_1.nbt"
            )),
            tags_26_2: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_26_2.nbt"
            )),
            tags_26_3: read_gzip_compound(include_bytes!(
                "../resources/dimension/tags_26_3.nbt"
            )),
        }
    }

    /// Port of getRegistryByVersion: pick the right codec compound for the version.
    fn get_registry_by_version(&self, version: Version) -> &NbtCompound {
        if version.more_or_equal(Version::V26_3) {
            &self.codec_26_3
        } else if version == Version::V26_2 {
            &self.codec_26_2
        } else if version == Version::V26_1 {
            &self.codec_26_1
        } else if version.more_or_equal(Version::V1_21_11) {
            &self.codec_1_21_11
        } else if version == Version::V1_21_9 {
            &self.codec_1_21_9
        } else if version == Version::V1_21_7 {
            &self.codec_1_21_7
        } else if version == Version::V1_21_6 {
            &self.codec_1_21_6
        } else if version == Version::V1_21_5 {
            &self.codec_1_21_5
        } else if version == Version::V1_21_4 {
            &self.codec_1_21_4
        } else if version == Version::V1_21_2 {
            &self.codec_1_21_2
        } else if version == Version::V1_21 {
            &self.codec_1_21
        } else if version == Version::V1_20_5 {
            &self.codec_1_20_5
        } else if version.more_or_equal(Version::V1_20) {
            &self.codec_1_20
        } else if version == Version::V1_19_4 {
            &self.codec_1_19_4
        } else if version.more_or_equal(Version::V1_19_1) {
            &self.codec_1_19_1
        } else if version == Version::V1_19 {
            &self.codec_1_19
        } else if version == Version::V1_18_2 {
            &self.codec_1_18_2
        } else if version.more_or_equal(Version::V1_17) {
            &self.codec_1_17
        } else if version.more_or_equal(Version::V1_16_2) {
            &self.codec_1_16_2
        } else {
            &self.codec_1_16
        }
    }

    /// findDimension: modern (`element`) then legacy fallback, defaulting to the first.
    ///
    /// Port of Java findDimension: tries the modern path (reads "id" and "element" compound)
    /// first, then falls back to the legacy path (uses list index as id, reads "height"
    /// directly from the dimension tag). Returns the first dimension if none match.
    pub fn find_dimension(&self, version: Version, key: &NamespacedKey) -> Option<Dimension> {
        let codec = self.get_registry_by_version(version);
        let key_str = key.to_string();

        // Modern path: dimension has an "element" sub-compound with the height.
        let modern = Self::find_default_dimension(codec, |_index, dimension_tag| {
            let name = dimension_tag.get_string("name")?;
            if name != &key_str {
                return None;
            }
            let element_tag = dimension_tag.get_compound("element")?;
            let id = dimension_tag.get_int("id").unwrap_or(0);
            let height = element_tag.get_int("height").unwrap_or(0);
            Some(Dimension {
                key: key.clone(),
                id,
                height,
                codec: codec.clone(),
                default_codec: element_tag.clone(),
            })
        });

        if modern.is_some() {
            return modern;
        }

        // Legacy path: list index is the id, height lives directly in the dimension tag.
        Self::find_default_dimension(codec, |index, dimension_tag| {
            let name = dimension_tag.get_string("name")?;
            if name != &key_str {
                return None;
            }
            let height = dimension_tag.get_int("height").unwrap_or(0);
            Some(Dimension {
                key: key.clone(),
                id: index as i32,
                height,
                codec: codec.clone(),
                default_codec: dimension_tag.clone(),
            })
        })
    }

    /// Port of findDefaultDimension: extract the dimension list from the codec, iterate it,
    /// apply `f`, and if nothing matched return the result for index 0 (always Some for modern
    /// path once an element is present, so we still call `f` on index 0 for the default).
    ///
    /// Returns `None` only when the list is empty or `f` never returns `Some`.
    fn find_default_dimension<F>(codec: &NbtCompound, f: F) -> Option<Dimension>
    where
        F: Fn(usize, &NbtCompound) -> Option<Dimension>,
    {
        // Modern codecs: "minecraft:dimension_type" is a Compound with a "value" list.
        // Legacy codecs: "dimension" is a list directly.
        let dimensions: &Vec<NbtTag> = match codec.get("minecraft:dimension_type") {
            Some(NbtTag::Compound(dim_type_compound)) => {
                dim_type_compound.get_list("value")?
            }
            _ => codec.get_list("dimension")?,
        };

        // Try each entry in order.
        for (i, tag) in dimensions.iter().enumerate() {
            if let NbtTag::Compound(dimension_tag) = tag
                && let Some(result) = f(i, dimension_tag)
            {
                return Some(result);
            }
        }

        // Default: apply f to the first entry.
        if let Some(NbtTag::Compound(first)) = dimensions.first() {
            return f(0, first);
        }

        None
    }

    /// The 1.20 codec used by PacketRegistryData for 1.20.2-1.20.3.
    pub fn codec_1_20(&self) -> &NbtCompound {
        &self.codec_1_20
    }

    /// Per-version registry metadata writers for versions >= 1.20.5.
    ///
    /// Port of createPerVersionRegistries: skips versions less than 1.20.5, builds
    /// a list of MetadataWriter closures for each qualifying version.
    pub fn create_per_version_registries(&self) -> HashMap<Version, Vec<MetadataWriter>> {
        let mut per_version: HashMap<Version, Vec<MetadataWriter>> = HashMap::new();

        for &version in Version::VALUES {
            if version.less(Version::V1_20_5) {
                continue;
            }
            let codec = self.get_registry_by_version(version);
            per_version.insert(version, Self::create_registries(codec));
        }

        per_version
    }

    /// Port of createRegistries: iterate top-level entries of the codec compound.
    /// Each entry is a registry type (String key) whose value is a Compound containing
    /// a "value" list.  One MetadataWriter closure is produced per entry.
    fn create_registries(tags: &NbtCompound) -> Vec<MetadataWriter> {
        let mut writers: Vec<MetadataWriter> = Vec::new();

        for (registry_type, tag) in &tags.child_tags {
            let compound_registry = match tag {
                NbtTag::Compound(c) => c,
                _ => continue,
            };

            let values = match compound_registry.get_list("value") {
                Some(v) => v.clone(),
                None => continue,
            };

            writers.push(Self::create_metadata_codec(registry_type.clone(), values));
        }

        writers
    }

    /// Port of createMetadataCodec: builds a MetadataWriter closure that, when called,
    /// writes the registry type and all its entries into a ByteMessage.
    fn create_metadata_codec(registry_type: String, values: Vec<NbtTag>) -> MetadataWriter {
        Box::new(move |msg, version| {
            msg.write_string(&registry_type);
            msg.write_var_int(values.len() as i32);

            for entry in &values {
                let entry_tag = match entry {
                    NbtTag::Compound(c) => c,
                    _ => continue,
                };

                let name = entry_tag.get_string("name").map(|s| s.as_str()).unwrap_or("");
                msg.write_string(name);

                // Elements are usually compounds, but not always: 26.3's
                // `minecraft:block_transformer` entries are lists. The wire format is
                // `Optional<Tag>` of any type, so write whatever tag is there.
                match entry_tag.get("element") {
                    Some(element_tag) => {
                        msg.write_bool(true);
                        msg.write_tag(element_tag, version);
                    }
                    None => {
                        msg.write_bool(false);
                    }
                }
            }
        })
    }

    /// Update-tags map for a version: type -> (tag -> [ids]).
    ///
    /// Port of createUpdateTags: select the right tags compound by version, then
    /// parse it via parseUpdateTags.
    pub fn create_update_tags(
        &self,
        version: Version,
    ) -> HashMap<String, HashMap<String, Vec<i32>>> {
        let tags = if version.more_or_equal(Version::V26_3) {
            &self.tags_26_3
        } else if version == Version::V26_2 {
            &self.tags_26_2
        } else if version == Version::V26_1 {
            &self.tags_26_1
        } else if version.more_or_equal(Version::V1_21_11) {
            &self.tags_1_21_11
        } else if version == Version::V1_21_9 {
            &self.tags_1_21_9
        } else if version == Version::V1_21_7 {
            &self.tags_1_21_7
        } else if version == Version::V1_21_6 {
            &self.tags_1_21_6
        } else if version == Version::V1_21_5 {
            &self.tags_1_21_5
        } else if version == Version::V1_21_4 {
            &self.tags_1_21_4
        } else if version == Version::V1_21_2 {
            &self.tags_1_21_2
        } else if version == Version::V1_21 {
            &self.tags_1_21
        } else {
            // Default for all versions < 1.21 (but >= 1.20.5, since tags only apply there)
            &self.tags_1_20_5
        };

        Self::parse_update_tags(tags)
    }

    /// Port of parseUpdateTags: walk the two-level compound tree and collect int lists.
    ///
    /// Structure: { registryType: { tagName: [IntTag, ...], ... }, ... }
    fn parse_update_tags(tags: &NbtCompound) -> HashMap<String, HashMap<String, Vec<i32>>> {
        let mut tags_map: HashMap<String, HashMap<String, Vec<i32>>> = HashMap::new();

        for (named_tag_key, named_tag_value) in &tags.child_tags {
            let sub_tag = match named_tag_value {
                NbtTag::Compound(c) => c,
                _ => continue,
            };

            let mut sub_tags_map: HashMap<String, Vec<i32>> = HashMap::new();

            for (sub_named_tag_key, sub_named_tag_value) in &sub_tag.child_tags {
                let ids_list = match sub_named_tag_value {
                    NbtTag::List(list) => list,
                    _ => continue,
                };

                let ids: Vec<i32> = ids_list
                    .iter()
                    .filter_map(|id_tag| match id_tag {
                        NbtTag::Int(v) => Some(*v),
                        _ => None,
                    })
                    .collect();

                sub_tags_map.insert(sub_named_tag_key.clone(), ids);
            }

            tags_map.insert(named_tag_key.clone(), sub_tags_map);
        }

        tags_map
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use hollow_protocol::ByteMessage;

    use super::*;
    use crate::DimensionType;

    const V26: [Version; 3] = [Version::V26_1, Version::V26_2, Version::V26_3];

    #[test]
    fn dimensions_resolve_for_26_x() {
        let registry = DimensionRegistry::load();
        for version in V26 {
            for (ty, id, height) in [
                (DimensionType::Overworld, 0, 384),
                (DimensionType::TheEnd, 2, 256),
                (DimensionType::TheNether, 3, 256),
            ] {
                let dim = registry.find_dimension(version, &ty.key()).unwrap();
                assert_eq!(dim.key, ty.key(), "{version:?}");
                assert_eq!((dim.id, dim.height), (id, height), "{ty:?} {version:?}");
            }
        }
    }

    #[test]
    fn each_26_x_version_gets_its_own_data() {
        let registry = DimensionRegistry::load();
        let has_registry =
            |v: Version, key: &str| registry.get_registry_by_version(v).get(key).is_some();
        let has_block_tag = |v: Version, tag: &str| {
            registry.create_update_tags(v)["minecraft:block"].contains_key(tag)
        };

        // Registries and block tags that first appear in 26.2 / 26.3.
        for (v, key, expected) in [
            (Version::V26_1, "minecraft:sulfur_cube_archetype", false),
            (Version::V26_2, "minecraft:sulfur_cube_archetype", true),
            (Version::V26_2, "minecraft:block_transformer", false),
            (Version::V26_3, "minecraft:block_transformer", true),
        ] {
            assert_eq!(has_registry(v, key), expected, "{key} on {v:?}");
        }
        for (v, tag, expected) in [
            (Version::V26_1, "minecraft:concrete", false),
            (Version::V26_2, "minecraft:concrete", true),
            (Version::V26_2, "minecraft:blocks_motion", false),
            (Version::V26_3, "minecraft:blocks_motion", true),
        ] {
            assert_eq!(has_block_tag(v, tag), expected, "{tag} on {v:?}");
        }

        // Vanilla synchronizes 28, 29 and 32 registries. 26.3's
        // minecraft:worldgen/block_state_provider is not sent: no synchronized vanilla
        // data references its entries (block_transformer providers are inline), so the
        // client's empty copy is never read.
        let counts = V26.map(|v| registry.get_registry_by_version(v).child_tags.len());
        assert_eq!(counts, [28, 29, 31]);
    }

    /// Decode what a RegistryData writer produced: (registry id, [(entry, Some(tag))]).
    fn decode_registry(bytes: &[u8]) -> (String, Vec<(String, Option<NbtTag>)>) {
        let mut msg = ByteMessage::from_bytes(bytes);
        let registry = msg.read_string().unwrap();
        let count = msg.read_var_int().unwrap();
        let mut entries = Vec::new();
        for _ in 0..count {
            let name = msg.read_string().unwrap();
            let data = if msg.read_bool().unwrap() {
                let mut cursor = Cursor::new(msg.as_read_slice());
                let tag = NbtTag::deserialize_from_cursor(&mut cursor).unwrap();
                msg.read_bytes(cursor.position() as usize).unwrap();
                Some(tag)
            } else {
                None
            };
            entries.push((name, data));
        }
        assert_eq!(msg.readable_bytes(), 0, "trailing bytes in {registry}");
        (registry, entries)
    }

    #[test]
    fn registry_data_round_trips_every_entry_with_data() {
        let registry = DimensionRegistry::load();
        let per_version = registry.create_per_version_registries();
        for (&version, writers) in &per_version {
            let codec = registry.get_registry_by_version(version);
            assert_eq!(writers.len(), codec.child_tags.len(), "{version:?}");
            for writer in writers {
                let mut msg = ByteMessage::new();
                writer(&mut msg, version);
                let (name, entries) = decode_registry(&msg.to_byte_array());

                let expected = codec
                    .get_compound(&name)
                    .unwrap()
                    .get_list("value")
                    .unwrap();
                assert_eq!(entries.len(), expected.len(), "{name} {version:?}");
                for ((entry, data), tag) in entries.iter().zip(expected) {
                    let NbtTag::Compound(tag) = tag else {
                        panic!("{name}: entry is not a compound")
                    };
                    assert_eq!(Some(entry), tag.get_string("name"), "{name} {version:?}");
                    assert_eq!(
                        data.as_ref(),
                        tag.get("element"),
                        "{name}/{entry} {version:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn block_transformer_list_elements_are_sent_on_26_3() {
        let registry = DimensionRegistry::load();
        let writers = &registry.create_per_version_registries()[&Version::V26_3];
        let transformers = writers
            .iter()
            .map(|w| {
                let mut msg = ByteMessage::new();
                w(&mut msg, Version::V26_3);
                decode_registry(&msg.to_byte_array())
            })
            .find(|(name, _)| name == "minecraft:block_transformer")
            .expect("26.3 syncs minecraft:block_transformer");
        let names: Vec<&str> = transformers.1.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            ["minecraft:axe", "minecraft:hoe", "minecraft:shovel"]
        );
        for (name, data) in &transformers.1 {
            assert!(
                matches!(data, Some(NbtTag::List(l)) if !l.is_empty()),
                "{name}"
            );
        }
    }

    #[test]
    fn update_tags_include_nested_tags() {
        // Vanilla resolves `#tag` references before sending; a tag must contain every id
        // of the tags it references (26.3's data originally shipped without them).
        let registry = DimensionRegistry::load();
        let pairs = [
            (
                "minecraft:block",
                "minecraft:logs",
                "minecraft:logs_that_burn",
            ),
            (
                "minecraft:block",
                "minecraft:logs_that_burn",
                "minecraft:oak_logs",
            ),
            (
                "minecraft:item",
                "minecraft:logs",
                "minecraft:logs_that_burn",
            ),
            (
                "minecraft:worldgen/biome",
                "minecraft:is_ocean",
                "minecraft:is_deep_ocean",
            ),
            (
                "minecraft:enchantment",
                "minecraft:on_random_loot",
                "minecraft:non_treasure",
            ),
        ];
        for version in V26 {
            let tags = registry.create_update_tags(version);
            for (reg, outer, inner) in pairs {
                let outer_ids = &tags[reg][outer];
                let inner_ids = &tags[reg][inner];
                assert!(!inner_ids.is_empty(), "{reg} {inner} {version:?}");
                for id in inner_ids {
                    assert!(
                        outer_ids.contains(id),
                        "{reg} {outer} lacks {inner} id {id} on {version:?}"
                    );
                }
            }
        }
    }
}
