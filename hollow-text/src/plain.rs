// SPDX-License-Identifier: GPL-3.0-only

//! Plain-text serializer, equivalent to PlainTextComponentSerializer (no translator).

use super::component::{Component, Content};

pub fn serialize(component: &Component) -> String {
    let mut out = String::new();
    append(component, &mut out);
    out
}

fn append(component: &Component, out: &mut String) {
    match &component.content {
        Content::Text(t) => out.push_str(t),
        Content::Translatable { key, .. } => out.push_str(key),
        Content::Keybind(k) => out.push_str(k),
        Content::Selector { pattern, .. } => out.push_str(pattern),
        Content::Score { value, .. } => {
            if let Some(v) = value {
                out.push_str(v);
            }
        }
        Content::Nbt { .. } => {}
        Content::Sprite { .. } => {}
        Content::Virtual => {}
    }
    for child in &component.children {
        append(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_children() {
        let mut c = Component::text("Hello, ");
        c.children.push(Component::text("World"));
        c.children.push(Component::text("!"));
        assert_eq!(serialize(&c), "Hello, World!");
    }
}
