// SPDX-License-Identifier: GPL-3.0-only

//! Text subsystem: a Rust port of the parts of kyori-adventure used by Hollow.
//!
//! Covers the component model, named/RGB colors with pre-1.16 downsampling, the
//! gson component serializer (4 version profiles), legacy `&`/`§` codes, plain text,
//! and a full MiniMessage parser/serializer.

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::large_enum_variant)]

pub mod color;
pub mod component;
pub mod gson;
pub mod legacy;
pub mod minimessage;
pub mod plain;

pub use color::{Color, NamedColor};
pub use component::{Component, Content, Decoration, Style};

/// Serialize a chat component into a packet buffer.
///
/// Lives here (rather than on `ByteMessage`) so the protocol crate need not depend on
/// the text crate: NBT tag for >= 1.20.3, JSON string for older versions.
pub fn write_component(
    buf: &mut hollow_protocol::ByteMessage,
    component: &Component,
    version: hollow_protocol::registry::Version,
) {
    use hollow_protocol::registry::Version;
    let profile = gson::profile_for(version);
    if version.more_or_equal(Version::V1_20_3) {
        let value = gson::to_value(component, profile);
        let compound = hollow_protocol::nbt::from_json(&value);
        buf.write_compound_tag(&compound, version);
    } else {
        let s = gson::serialize(component, profile);
        buf.write_string(&s);
    }
}

/// Port of ComponentUtils.parse: JSON, then legacy/MiniMessage handling.
pub fn parse(text: &str) -> Component {
    if text.is_empty() {
        return Component::empty();
    }

    // Old json-like deserialization.
    if hollow_protocol::json::is_valid_json(text) {
        let replaced = text.replace('&', "\u{00A7}");
        if let Some(c) = gson::deserialize(&replaced) {
            return c;
        }
    }

    let mut t = text.to_string();
    if t.contains('\u{00A7}') {
        t = t.replace('\u{00A7}', "&");
    }

    if !t.contains('&') {
        return minimessage::deserialize(&t);
    }

    // Mixed legacy + MiniMessage: legacy -> component -> MiniMessage string -> re-parse.
    let legacy_comp = legacy::deserialize(&t);
    let mm = minimessage::serialize(&legacy_comp)
        .replace("\\<", "<")
        .replace("\\>", ">");
    minimessage::deserialize(&mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_config_strings() {
        // <white>Welcome! -> white text
        let c = parse("<white>Welcome!");
        assert_eq!(
            gson::serialize(&c, gson::MODERN),
            "{\"color\":\"white\",\"text\":\"Welcome!\"}"
        );

        // Empty -> empty component
        assert_eq!(gson::serialize(&parse(""), gson::MODERN), "\"\"");

        // JSON input parsed as component
        let c = parse("{\"text\":\"hi\",\"bold\":true}");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"text\":\"hi\""), "json={json}");
        assert!(json.contains("\"bold\":true"), "json={json}");
    }

    #[test]
    fn parse_gradient_brand() {
        // The default brandName / ping value.
        let c = parse("<gradient:blue:white>Hollow");
        // gradient: each char gets its own color; legacy section form downsamples to named codes.
        let legacy = legacy::serialize_section(&c);
        assert!(legacy.starts_with('\u{00A7}'), "legacy={legacy:?}");
        // plain text round-trips to the literal content
        assert_eq!(plain::serialize(&c), "Hollow");
    }
}
