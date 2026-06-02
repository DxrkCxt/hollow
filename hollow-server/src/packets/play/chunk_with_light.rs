// SPDX-License-Identifier: GPL-3.0-only

use crab_nbt::{NbtCompound, NbtTag};

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_world::VersionedDimension;

// HeightMapType ordinal values match Java's enum declaration order.
const HEIGHTMAP_MOTION_BLOCKING_ORDINAL: i32 = 4;

#[derive(Clone, Debug)]
pub struct PacketChunkWithLight {
    pub x: i32,
    pub z: i32,
    pub dimension: VersionedDimension,
}

impl Default for PacketChunkWithLight {
    fn default() -> Self {
        panic!("PacketChunkWithLight requires a dimension; use PacketChunkWithLight::new()")
    }
}

impl PacketChunkWithLight {
    pub fn new(x: i32, z: i32, dimension: VersionedDimension) -> Self {
        PacketChunkWithLight { x, z, dimension }
    }
}

impl PacketOut for PacketChunkWithLight {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        buf.write_i32(self.x);
        buf.write_i32(self.z);

        write_heightmaps(buf, version);
        self.write_blocks_data(buf, version);

        // Skip block entities
        buf.write_var_int(0);

        self.write_light_data(buf, version);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::ChunkWithLight
    }
}

impl PacketChunkWithLight {
    fn write_blocks_data(&self, buf: &mut ByteMessage, version: Version) {
        let sections = self.dimension.chunk_sections(version);
        let section_bytes = create_empty_section_bytes(version);
        let section_len = section_bytes.len() as i32;
        buf.write_var_int(section_len * sections);
        for _ in 0..sections {
            buf.write_bytes(&section_bytes);
        }
    }

    fn write_light_data(&self, buf: &mut ByteMessage, version: Version) {
        // Sky y mask: null BitSet → writeVarInt(0)
        buf.write_var_int(0);
        // Block y mask: null BitSet → writeVarInt(0)
        buf.write_var_int(0);
        // Empty sky y mask: null BitSet → writeVarInt(0)
        buf.write_var_int(0);

        // Empty block y mask: BitSet with bits 0..=(sections+1) set
        let sections = self.dimension.chunk_sections(version);
        let num_bits = (sections + 2) as usize;
        write_bitset_all_set(buf, num_bits);

        // Skip sky light arrays
        buf.write_var_int(0);
        // Skip block light arrays
        buf.write_var_int(0);
    }
}

/// Write a BitSet where the first `num_bits` bits are all set to true,
/// serialized as a long array (Java BitSet.toLongArray() format: LSB-first packed longs).
fn write_bitset_all_set(buf: &mut ByteMessage, num_bits: usize) {
    if num_bits == 0 {
        buf.write_var_int(0);
        return;
    }
    let num_longs = num_bits.div_ceil(64);
    buf.write_var_int(num_longs as i32);
    for i in 0..num_longs {
        let bits_in_this_long = if (i + 1) * 64 <= num_bits {
            64
        } else {
            num_bits - i * 64
        };
        let mask: i64 = if bits_in_this_long == 64 {
            -1i64 // all bits set
        } else {
            (1i64 << bits_in_this_long) - 1
        };
        buf.write_i64(mask);
    }
}

/// Build the bytes for one empty chunk section, mirroring Java's createEmptySection.
fn create_empty_section_bytes(version: Version) -> Vec<u8> {
    let mut sec = ByteMessage::new();

    sec.write_i16(0); // non-air block count

    if version.more_or_equal(Version::V26_1) {
        sec.write_i16(0); // fluid count (new in 26.1)
    }

    // blocks palette (SinglePaletteFactory(0))
    write_palette(&mut sec, version, 0, 0);
    // biomes palette (SinglePaletteFactory(0))
    write_palette(&mut sec, version, 0, 0);

    sec.to_byte_array()
}

/// Port of writePalette(buf, version, paletteFactory, storage=[]).
/// `palette_id` is the byte written as the palette type (0 = single value),
/// `palette_data` is the VarInt written as the palette's data value.
fn write_palette(buf: &mut ByteMessage, version: Version, palette_id: u8, palette_data: i32) {
    buf.write_u8(palette_id);
    buf.write_var_int(palette_data);

    // Storage array is always empty (long[0])
    if version.more_or_equal(Version::V1_21_5) {
        // No length prefix in 1.21.5+
        // (no longs to write)
    } else {
        buf.write_var_int(0); // storage.length = 0
        // (no longs to write)
    }
}

/// Port of writeHeightmaps: for >=1.21.5 write VarInt-based map; otherwise write as NBT compound.
/// We always send one heightmap: MOTION_BLOCKING with 37 zero longs.
fn write_heightmaps(buf: &mut ByteMessage, version: Version) {
    if version.more_or_equal(Version::V1_21_5) {
        buf.write_var_int(1); // 1 entry
        buf.write_var_int(HEIGHTMAP_MOTION_BLOCKING_ORDINAL); // key = MOTION_BLOCKING ordinal (4)
        // long array: 37 zero longs
        buf.write_var_int(37); // length
        for _ in 0..37 {
            buf.write_i64(0);
        }
    } else {
        // Build NBT: { "": { "root": { "MOTION_BLOCKING": [0L x 37] } } }
        // Java code:
        //   CompoundBinaryTag inner = CompoundBinaryTag.builder()
        //       .put("MOTION_BLOCKING", LongArrayBinaryTag.longArrayBinaryTag(new long[37]))
        //       .build();
        //   CompoundBinaryTag root = CompoundBinaryTag.builder()
        //       .put("root", inner)
        //       .build();
        //   buf.writeCompoundTag(root, version);
        let longs: Vec<i64> = vec![0i64; 37];
        let mut inner = NbtCompound::new();
        inner.child_tags.push(("MOTION_BLOCKING".to_string(), NbtTag::LongArray(longs)));
        let mut root = NbtCompound::new();
        root.child_tags.push(("root".to_string(), NbtTag::Compound(inner)));
        buf.write_compound_tag(&root, version);
    }
}
