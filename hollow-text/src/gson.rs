// SPDX-License-Identifier: GPL-3.0-only

//! gson component (de)serializer with the four version profiles used by Hollow.
//!
//! Serialization order and option handling are ported from adventure's
//! ComponentSerializerImpl / StyleSerializer so output matches Java byte-for-byte.
//! JSON output uses serde_json with `preserve_order` (insertion-ordered) and matches
//! Gson's `disableHtmlEscaping()` escaping.

use serde_json::{Map, Value};

use super::color::{Color, NamedColor};
use super::component::{
    ClickAction, ClickEvent, Component, Content, Decoration, HoverEvent, NbtSource, Style,
};
use hollow_protocol::registry::Version;

/// Flag set derived from JSONOptions for a given protocol era.
#[derive(Clone, Copy)]
pub struct Profile {
    pub emit_rgb: bool,
    pub compact_text: bool,
    pub click_camel: bool,
    pub click_snake: bool,
    pub hover_value_field: bool,
    pub hover_camel: bool,
    pub hover_snake: bool,
    pub string_page: bool,
    pub click_url_https: bool,
}

pub const PRE_1_16: Profile = Profile {
    emit_rgb: false,
    compact_text: false,
    click_camel: true,
    click_snake: false,
    hover_value_field: true,
    hover_camel: false,
    hover_snake: false,
    string_page: true,
    click_url_https: true,
};

pub const PRE_1_20_3: Profile = Profile {
    emit_rgb: true,
    compact_text: false,
    click_camel: true,
    click_snake: false,
    hover_value_field: false,
    hover_camel: true,
    hover_snake: false,
    string_page: true,
    click_url_https: true,
};

pub const PRE_1_21_5: Profile = Profile {
    emit_rgb: true,
    compact_text: true,
    click_camel: true,
    click_snake: false,
    hover_value_field: false,
    hover_camel: true,
    hover_snake: false,
    string_page: true,
    click_url_https: true,
};

pub const MODERN: Profile = Profile {
    emit_rgb: true,
    compact_text: true,
    click_camel: false,
    click_snake: true,
    hover_value_field: false,
    hover_camel: false,
    hover_snake: true,
    string_page: false,
    click_url_https: true,
};

/// Equivalent to ComponentUtils.getJsonChatSerializer(version).
pub fn profile_for(version: Version) -> Profile {
    if version.more_or_equal(Version::V1_21_5) {
        MODERN
    } else if version.more_or_equal(Version::V1_20_3) {
        PRE_1_21_5
    } else if version.more_or_equal(Version::V1_16) {
        PRE_1_20_3
    } else {
        PRE_1_16
    }
}

/// Serialize to a compact JSON string (Gson-compatible).
pub fn serialize(component: &Component, profile: Profile) -> String {
    to_value(component, profile).to_string()
}

/// Serialize to a serde_json value (needed for component-as-NBT on 1.20.3+).
pub fn to_value(component: &Component, profile: Profile) -> Value {
    // Compact text component: bare string.
    if profile.compact_text
        && component.children.is_empty()
        && !component.has_styling()
        && let Content::Text(s) = &component.content
    {
        return Value::String(s.clone());
    }

    let mut map = Map::new();

    // 1) style fields first
    if component.has_styling() {
        write_style(&component.style, profile, &mut map);
    }

    // 2) extra (children)
    if !component.children.is_empty() {
        let arr: Vec<Value> = component
            .children
            .iter()
            .map(|c| to_value(c, profile))
            .collect();
        map.insert("extra".into(), Value::Array(arr));
    }

    // 3) content
    match &component.content {
        Content::Text(s) => {
            map.insert("text".into(), Value::String(s.clone()));
        }
        Content::Translatable {
            key,
            fallback,
            with,
        } => {
            map.insert("translate".into(), Value::String(key.clone()));
            if let Some(fb) = fallback {
                map.insert("fallback".into(), Value::String(fb.clone()));
            }
            if !with.is_empty() {
                let arr: Vec<Value> = with.iter().map(|c| to_value(c, profile)).collect();
                map.insert("with".into(), Value::Array(arr));
            }
        }
        Content::Keybind(k) => {
            map.insert("keybind".into(), Value::String(k.clone()));
        }
        Content::Score {
            name,
            objective,
            value,
        } => {
            let mut score = Map::new();
            score.insert("name".into(), Value::String(name.clone()));
            score.insert("objective".into(), Value::String(objective.clone()));
            if let Some(v) = value {
                score.insert("value".into(), Value::String(v.clone()));
            }
            map.insert("score".into(), Value::Object(score));
        }
        Content::Selector { pattern, separator } => {
            map.insert("selector".into(), Value::String(pattern.clone()));
            if let Some(sep) = separator {
                map.insert("separator".into(), to_value(sep, profile));
            }
        }
        Content::Nbt {
            kind,
            path,
            interpret,
            separator,
        } => {
            map.insert("nbt".into(), Value::String(path.clone()));
            map.insert("interpret".into(), Value::Bool(*interpret));
            if let Some(sep) = separator {
                map.insert("separator".into(), to_value(sep, profile));
            }
            match kind {
                NbtSource::Block(b) => {
                    map.insert("block".into(), Value::String(b.clone()));
                }
                NbtSource::Entity(e) => {
                    map.insert("entity".into(), Value::String(e.clone()));
                }
                NbtSource::Storage(s) => {
                    map.insert("storage".into(), Value::String(s.clone()));
                }
            }
        }
        Content::Sprite { atlas, sprite } => {
            if let Some(atlas) = atlas {
                map.insert("atlas".into(), Value::String(atlas.clone()));
            }
            map.insert("sprite".into(), Value::String(sprite.clone()));
        }
        // A virtual wrapper serializes like an empty text component.
        Content::Virtual => {
            map.insert("text".into(), Value::String(String::new()));
        }
    }

    Value::Object(map)
}

fn color_value(color: Color, profile: Profile) -> Value {
    if !profile.emit_rgb {
        Value::String(color.nearest_named().name().to_string())
    } else {
        match color {
            Color::Named(n) => Value::String(n.name().to_string()),
            // gson emits non-named colors as UPPERCASE hex (asUpperCaseHexString: "#%06X").
            Color::Rgb(_) => Value::String(format!("#{:06X}", color.value())),
        }
    }
}

fn write_style(style: &Style, profile: Profile, map: &mut Map<String, Value>) {
    for d in Decoration::DECORATIONS {
        if let Some(v) = style.decoration(d) {
            map.insert(d.name().to_string(), Value::Bool(v));
        }
    }
    if let Some(color) = style.color {
        map.insert("color".to_string(), color_value(color, profile));
    }
    if let Some(shadow) = style.shadow_color {
        // All four profiles use SHADOW_COLOR_MODE = EMIT_INTEGER (packed ARGB integer).
        map.insert("shadow_color".to_string(), Value::Number((shadow as i64).into()));
    }
    if let Some(ins) = &style.insertion {
        map.insert("insertion".to_string(), Value::String(ins.clone()));
    }
    if let Some(click) = &style.click {
        write_click(click, profile, map);
    }
    if let Some(hover) = &style.hover {
        write_hover(hover, profile, map);
    }
    if let Some(font) = &style.font {
        map.insert("font".to_string(), Value::String(font.clone()));
    }
}

fn maybe_https(action: ClickAction, value: &str, profile: Profile) -> String {
    if action == ClickAction::OpenUrl && profile.click_url_https && !is_valid_url_scheme(value) {
        format!("https://{value}")
    } else {
        value.to_string()
    }
}

fn is_valid_url_scheme(value: &str) -> bool {
    // adventure checks for "https://" or "http://" prefix (case-insensitive scheme).
    let lower = value.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

fn write_click(click: &ClickEvent, profile: Profile, map: &mut Map<String, Value>) {
    let action = click.action;
    if profile.click_snake {
        let mut obj = Map::new();
        obj.insert("action".into(), Value::String(action.name().into()));
        if action.is_text_payload() {
            let field = match action {
                ClickAction::OpenUrl => "url",
                ClickAction::RunCommand | ClickAction::SuggestCommand => "command",
                ClickAction::CopyToClipboard => "value",
                // open_file has no dedicated field in adventure's switch; "value" is a safe fallback
                _ => "value",
            };
            obj.insert(
                field.into(),
                Value::String(maybe_https(action, &click.value, profile)),
            );
        } else if action == ClickAction::ChangePage {
            if profile.string_page {
                obj.insert("page".into(), Value::String(click.value.clone()));
            } else if let Ok(n) = click.value.parse::<i64>() {
                obj.insert("page".into(), Value::Number(n.into()));
            }
        }
        map.insert("click_event".into(), Value::Object(obj));
    }
    if profile.click_camel && action.is_text_payload() {
        let mut obj = Map::new();
        obj.insert("action".into(), Value::String(action.name().into()));
        obj.insert(
            "value".into(),
            Value::String(maybe_https(action, &click.value, profile)),
        );
        map.insert("clickEvent".into(), Value::Object(obj));
    }
}

fn hover_action_name(hover: &HoverEvent) -> &'static str {
    match hover {
        HoverEvent::ShowText(_) => "show_text",
        HoverEvent::ShowItem { .. } => "show_item",
        HoverEvent::ShowEntity { .. } => "show_entity",
    }
}

fn hover_contents(hover: &HoverEvent, profile: Profile) -> Value {
    match hover {
        HoverEvent::ShowText(c) => to_value(c, profile),
        HoverEvent::ShowItem { id, count } => {
            let mut obj = Map::new();
            obj.insert("id".into(), Value::String(id.clone()));
            if let Some(c) = count {
                obj.insert("count".into(), Value::Number((*c as i64).into()));
            }
            Value::Object(obj)
        }
        HoverEvent::ShowEntity { id_type, id, name } => {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String(id_type.clone()));
            obj.insert("id".into(), Value::String(id.clone()));
            if let Some(n) = name {
                obj.insert("name".into(), to_value(n, profile));
            }
            Value::Object(obj)
        }
    }
}

fn write_hover(hover: &HoverEvent, profile: Profile, map: &mut Map<String, Value>) {
    let action = hover_action_name(hover);
    if profile.hover_snake {
        let mut obj = Map::new();
        obj.insert("action".into(), Value::String(action.into()));
        match hover {
            HoverEvent::ShowText(c) => {
                obj.insert("value".into(), to_value(c, profile));
            }
            other => {
                if let Value::Object(fields) = hover_contents(other, profile) {
                    for (k, v) in fields {
                        obj.insert(k, v);
                    }
                }
            }
        }
        map.insert("hover_event".into(), Value::Object(obj));
    }
    if profile.hover_camel || profile.hover_value_field {
        let mut obj = Map::new();
        obj.insert("action".into(), Value::String(action.into()));
        if profile.hover_camel {
            obj.insert("contents".into(), hover_contents(hover, profile));
        }
        if profile.hover_value_field {
            // Legacy "value" field: for show_text this is the component itself.
            obj.insert("value".into(), hover_contents(hover, profile));
        }
        map.insert("hoverEvent".into(), Value::Object(obj));
    }
}

// ---------------------------------------------------------------------------
// Deserialization (JSON -> Component), used by the `parse()` JSON branch.
// ---------------------------------------------------------------------------

pub fn deserialize(json: &str) -> Option<Component> {
    let value: Value = serde_json::from_str(json).ok()?;
    from_value(&value)
}

pub fn from_value(value: &Value) -> Option<Component> {
    match value {
        Value::String(s) => Some(Component::text(s.clone())),
        Value::Array(arr) => {
            // A bare array: first element is the parent, rest are children.
            let mut it = arr.iter();
            let first = it.next()?;
            let mut comp = from_value(first)?;
            for child in it {
                if let Some(c) = from_value(child) {
                    comp.children.push(c);
                }
            }
            Some(comp)
        }
        Value::Object(obj) => Some(from_object(obj)),
        Value::Bool(b) => Some(Component::text(b.to_string())),
        Value::Number(n) => Some(Component::text(n.to_string())),
        Value::Null => None,
    }
}

fn from_object(obj: &Map<String, Value>) -> Component {
    let content = if let Some(Value::String(s)) = obj.get("text") {
        Content::Text(s.clone())
    } else if let Some(Value::String(key)) = obj.get("translate") {
        let fallback = obj.get("fallback").and_then(|v| v.as_str()).map(String::from);
        let with = obj
            .get("with")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(from_value).collect())
            .unwrap_or_default();
        Content::Translatable {
            key: key.clone(),
            fallback,
            with,
        }
    } else if let Some(Value::String(k)) = obj.get("keybind") {
        Content::Keybind(k.clone())
    } else if let Some(Value::Object(score)) = obj.get("score") {
        Content::Score {
            name: score.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            objective: score
                .get("objective")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            value: score.get("value").and_then(|v| v.as_str()).map(String::from),
        }
    } else if let Some(Value::String(p)) = obj.get("selector") {
        let separator = obj.get("separator").and_then(from_value).map(Box::new);
        Content::Selector {
            pattern: p.clone(),
            separator,
        }
    } else {
        Content::Text(String::new())
    };

    let mut style = Style::default();
    if let Some(Value::String(c)) = obj.get("color") {
        style.color = parse_color(c);
    }
    for d in Decoration::DECORATIONS {
        if let Some(v) = obj.get(d.name()).and_then(|v| v.as_bool()) {
            style.set_decoration(d, Some(v));
        }
    }
    if let Some(Value::String(ins)) = obj.get("insertion") {
        style.insertion = Some(ins.clone());
    }
    if let Some(Value::String(f)) = obj.get("font") {
        style.font = Some(f.clone());
    }

    let mut children = Vec::new();
    if let Some(Value::Array(extra)) = obj.get("extra") {
        for c in extra {
            if let Some(comp) = from_value(c) {
                children.push(comp);
            }
        }
    }

    Component {
        content,
        style,
        children,
    }
}

fn parse_color(s: &str) -> Option<Color> {
    if let Some(stripped) = s.strip_prefix('#') {
        return Color::from_hex(stripped);
    }
    NamedColor::from_name(s).map(Color::Named)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styled(color: Color, bold: bool) -> Component {
        let mut c = Component::text("hi");
        c.style.color = Some(color);
        if bold {
            c.style.bold = Some(true);
        }
        c
    }

    #[test]
    fn compact_leaf_modern() {
        let c = Component::text("hello");
        assert_eq!(serialize(&c, MODERN), "\"hello\"");
    }

    #[test]
    fn non_compact_pre_1_16() {
        let c = Component::text("hello");
        // pre-1.16 is not compact -> object form
        assert_eq!(serialize(&c, PRE_1_16), "{\"text\":\"hello\"}");
    }

    #[test]
    fn style_before_text_named_color() {
        let c = styled(Color::Named(NamedColor::Red), true);
        // order: bold (decoration), color, then text
        assert_eq!(
            serialize(&c, MODERN),
            "{\"bold\":true,\"color\":\"red\",\"text\":\"hi\"}"
        );
    }

    #[test]
    fn rgb_downsample_pre_1_16() {
        // pure blue rgb -> nearest named "blue"
        let c = styled(Color::Rgb(0x5555FF), false);
        assert_eq!(serialize(&c, PRE_1_16), "{\"color\":\"blue\",\"text\":\"hi\"}");
    }

    #[test]
    fn rgb_hex_modern() {
        let c = styled(Color::Rgb(0x123456), false);
        assert_eq!(
            serialize(&c, MODERN),
            "{\"color\":\"#123456\",\"text\":\"hi\"}"
        );
    }

    #[test]
    fn extra_then_text() {
        let mut c = Component::text("a");
        c.children.push(Component::text("b"));
        // no styling: order is extra then text; children compact strings under MODERN
        assert_eq!(serialize(&c, MODERN), "{\"extra\":[\"b\"],\"text\":\"a\"}");
    }
}
