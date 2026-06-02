// SPDX-License-Identifier: GPL-3.0-only

//! NBT helpers: gzip codec loading, JSON->NBT (NbtUtils.fromJson), versioned writing.

use std::io::Read;

use bytes::Bytes;
use crab_nbt::{Nbt, NbtCompound, NbtTag};
use flate2::read::GzDecoder;
use serde_json::Value;

use crate::registry::Version;

/// Read a GZIP-compressed Java NBT resource into its root compound.
pub fn read_gzip_compound(data: &[u8]) -> NbtCompound {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .expect("failed to gunzip NBT resource");
    let mut buf = Bytes::from(out);
    Nbt::read(&mut buf)
        .expect("failed to read NBT resource")
        .root_tag
}

/// Serialize a compound as network NBT (nameless root) for 1.20.2+, otherwise as a
/// named root with an empty name — matching ByteMessage.writeCompoundTag(tag, version).
pub fn write_compound(compound: &NbtCompound, version: Version) -> Bytes {
    let nbt = Nbt::new(String::new(), compound.clone());
    if version.more_or_equal(Version::V1_20_2) {
        nbt.write_unnamed()
    } else {
        nbt.write()
    }
}

/// Port of NbtUtils.fromJson: a non-object value is wrapped as `{ "text": value }`.
pub fn from_json(value: &Value) -> NbtCompound {
    if let Value::Object(map) = value {
        object_to_compound(map)
    } else {
        let mut c = NbtCompound::new();
        c.child_tags.push(("text".to_string(), value_to_tag(value)));
        c
    }
}

fn object_to_compound(map: &serde_json::Map<String, Value>) -> NbtCompound {
    let mut c = NbtCompound::new();
    for (k, v) in map {
        c.child_tags.push((k.clone(), value_to_tag(v)));
    }
    c
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Long mirrors NbtUtils but serde_json never yields a long-typed number
enum NType {
    End,
    Byte,
    Int,
    Long,
    Double,
    String,
    List,
    Compound,
    ByteArray,
    IntArray,
    LongArray,
}

fn nbt_type_of(value: &Value) -> NType {
    match value {
        Value::Null => NType::End,
        Value::Bool(_) => NType::Byte,
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                NType::Int
            } else {
                NType::Double
            }
        }
        Value::String(_) => NType::String,
        Value::Object(_) => NType::Compound,
        Value::Array(arr) => match unify_array(arr) {
            None => NType::List,
            Some(NType::Byte) => NType::ByteArray,
            Some(NType::Int) => NType::IntArray,
            Some(NType::Long) => NType::LongArray,
            Some(_) => NType::List,
        },
    }
}

fn unify_array(arr: &[Value]) -> Option<NType> {
    let mut list_type: Option<NType> = None;
    for el in arr {
        let t = nbt_type_of(el);
        match list_type {
            None => list_type = Some(t),
            Some(prev) => {
                if prev != t {
                    list_type = Some(NType::Compound);
                    break;
                }
            }
        }
    }
    list_type
}

fn value_to_tag(value: &Value) -> NbtTag {
    match value {
        Value::Null => NbtTag::End,
        Value::Bool(b) => NbtTag::Byte(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                NbtTag::Int(i as i32)
            } else if let Some(u) = n.as_u64() {
                NbtTag::Int(u as i32)
            } else {
                NbtTag::Double(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => NbtTag::String(s.clone()),
        Value::Object(map) => NbtTag::Compound(object_to_compound(map)),
        Value::Array(arr) => array_to_tag(arr),
    }
}

fn array_to_tag(arr: &[Value]) -> NbtTag {
    let list_type = unify_array(arr);
    match list_type {
        None | Some(NType::End) => NbtTag::List(Vec::new()),
        Some(NType::Byte) => {
            let bytes: Vec<u8> = arr
                .iter()
                .map(|v| v.as_i64().unwrap_or(0) as u8)
                .collect();
            NbtTag::ByteArray(Bytes::from(bytes))
        }
        Some(NType::Int) => {
            let ints: Vec<i32> = arr.iter().map(|v| v.as_i64().unwrap_or(0) as i32).collect();
            NbtTag::IntArray(ints)
        }
        Some(NType::Long) => {
            let longs: Vec<i64> = arr.iter().map(|v| v.as_i64().unwrap_or(0)).collect();
            NbtTag::LongArray(longs)
        }
        Some(ty) => {
            let mut tags = Vec::with_capacity(arr.len());
            for el in arr {
                let mut tag = value_to_tag(el);
                if ty == NType::Compound && !matches!(tag, NbtTag::Compound(_)) {
                    let mut wrap = NbtCompound::new();
                    wrap.child_tags.push((String::new(), tag));
                    tag = NbtTag::Compound(wrap);
                }
                tags.push(tag);
            }
            NbtTag::List(tags)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn primitive_wrapped_in_text() {
        let c = from_json(&json!("hi"));
        assert_eq!(c.child_tags.len(), 1);
        assert_eq!(c.child_tags[0].0, "text");
        assert!(matches!(c.child_tags[0].1, NbtTag::String(_)));
    }

    #[test]
    fn object_fields_preserve_order() {
        let c = from_json(&json!({"text": "hi", "bold": true}));
        assert_eq!(c.child_tags[0].0, "text");
        assert_eq!(c.child_tags[1].0, "bold");
        assert!(matches!(c.child_tags[1].1, NbtTag::Byte(1)));
    }
}
