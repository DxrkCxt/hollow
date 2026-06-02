// SPDX-License-Identifier: GPL-3.0-only

//! Component model, mirroring the kyori-adventure component/style shape closely
//! enough to reproduce its gson JSON output and MiniMessage handling.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::NamedColor;

    #[test]
    fn compact_empty_parent_single_child() {
        let mut parent = Component::text("");
        parent.style.color = Some(Color::Named(NamedColor::White));
        parent.children.push(Component::text("Welcome!"));
        let c = parent.compact();
        assert_eq!(c.content, Content::Text("Welcome!".into()));
        assert_eq!(c.style.color, Some(Color::Named(NamedColor::White)));
        assert!(c.children.is_empty());
    }

    #[test]
    fn compact_merges_unstyled_text_children() {
        let mut parent = Component::text("");
        parent.children.push(Component::text("a"));
        parent.children.push(Component::text("b"));
        let c = parent.compact();
        assert_eq!(c.content, Content::Text("ab".into()));
        assert!(c.children.is_empty());
    }
}

use super::color::Color;

/// Text decorations in adventure's `DecorationMap` / serialization order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decoration {
    Obfuscated,
    Bold,
    Strikethrough,
    Underlined,
    Italic,
}

impl Decoration {
    /// Same order the gson StyleSerializer iterates (TextDecoration enum order).
    pub const DECORATIONS: [Decoration; 5] = [
        Decoration::Obfuscated,
        Decoration::Bold,
        Decoration::Strikethrough,
        Decoration::Underlined,
        Decoration::Italic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Decoration::Obfuscated => "obfuscated",
            Decoration::Bold => "bold",
            Decoration::Strikethrough => "strikethrough",
            Decoration::Underlined => "underlined",
            Decoration::Italic => "italic",
        }
    }

    pub fn from_name(name: &str) -> Option<Decoration> {
        Decoration::DECORATIONS
            .into_iter()
            .find(|d| d.name() == name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClickAction {
    OpenUrl,
    OpenFile,
    RunCommand,
    SuggestCommand,
    ChangePage,
    CopyToClipboard,
}

impl ClickAction {
    pub fn name(self) -> &'static str {
        match self {
            ClickAction::OpenUrl => "open_url",
            ClickAction::OpenFile => "open_file",
            ClickAction::RunCommand => "run_command",
            ClickAction::SuggestCommand => "suggest_command",
            ClickAction::ChangePage => "change_page",
            ClickAction::CopyToClipboard => "copy_to_clipboard",
        }
    }

    pub fn from_name(name: &str) -> Option<ClickAction> {
        Some(match name {
            "open_url" => ClickAction::OpenUrl,
            "open_file" => ClickAction::OpenFile,
            "run_command" => ClickAction::RunCommand,
            "suggest_command" => ClickAction::SuggestCommand,
            "change_page" => ClickAction::ChangePage,
            "copy_to_clipboard" => ClickAction::CopyToClipboard,
            _ => return None,
        })
    }

    /// Whether this action carries a textual (vs int/custom) payload.
    pub fn is_text_payload(self) -> bool {
        !matches!(self, ClickAction::ChangePage)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClickEvent {
    pub action: ClickAction,
    pub value: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HoverEvent {
    ShowText(Box<Component>),
    ShowItem {
        id: String,
        count: Option<i32>,
    },
    ShowEntity {
        id_type: String,
        id: String,
        name: Option<Box<Component>>,
    },
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Style {
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub insertion: Option<String>,
    pub click: Option<ClickEvent>,
    pub hover: Option<HoverEvent>,
    pub font: Option<String>,
    /// Packed ARGB shadow color (1.21.4+). `None` = unset; `Some(0)` = `<!shadow>` (none).
    pub shadow_color: Option<i32>,
}

impl Style {
    pub fn decoration(&self, d: Decoration) -> Option<bool> {
        match d {
            Decoration::Obfuscated => self.obfuscated,
            Decoration::Bold => self.bold,
            Decoration::Strikethrough => self.strikethrough,
            Decoration::Underlined => self.underlined,
            Decoration::Italic => self.italic,
        }
    }

    pub fn set_decoration(&mut self, d: Decoration, value: Option<bool>) {
        match d {
            Decoration::Obfuscated => self.obfuscated = value,
            Decoration::Bold => self.bold = value,
            Decoration::Strikethrough => self.strikethrough = value,
            Decoration::Underlined => self.underlined = value,
            Decoration::Italic => self.italic = value,
        }
    }

    /// Equivalent to Component.hasStyling(): any style field set.
    pub fn is_present(&self) -> bool {
        self.color.is_some()
            || self.bold.is_some()
            || self.italic.is_some()
            || self.underlined.is_some()
            || self.strikethrough.is_some()
            || self.obfuscated.is_some()
            || self.insertion.is_some()
            || self.click.is_some()
            || self.hover.is_some()
            || self.font.is_some()
            || self.shadow_color.is_some()
    }

    /// Fill any unset field of `self` from `parent` (child-inherits-parent), used by MiniMessage.
    pub fn inherit_from(&mut self, parent: &Style) {
        self.merge_if_absent(parent);
    }

    /// Adventure `merge(other, IF_ABSENT_ON_TARGET)`: take each unset field from `other`.
    pub fn merge_if_absent(&mut self, other: &Style) {
        if self.color.is_none() {
            self.color = other.color;
        }
        for d in Decoration::DECORATIONS {
            if self.decoration(d).is_none() {
                self.set_decoration(d, other.decoration(d));
            }
        }
        if self.insertion.is_none() {
            self.insertion = other.insertion.clone();
        }
        if self.click.is_none() {
            self.click = other.click.clone();
        }
        if self.hover.is_none() {
            self.hover = other.hover.clone();
        }
        if self.font.is_none() {
            self.font = other.font.clone();
        }
        if self.shadow_color.is_none() {
            self.shadow_color = other.shadow_color;
        }
    }

    /// Adventure `unmerge(parent)`: drop any field equal to the parent's.
    pub fn unmerge(&self, parent: &Style) -> Style {
        let mut r = self.clone();
        if r.color == parent.color {
            r.color = None;
        }
        for d in Decoration::DECORATIONS {
            if r.decoration(d) == parent.decoration(d) {
                r.set_decoration(d, None);
            }
        }
        if r.insertion == parent.insertion {
            r.insertion = None;
        }
        if r.click == parent.click {
            r.click = None;
        }
        if r.hover == parent.hover {
            r.hover = None;
        }
        if r.font == parent.font {
            r.font = None;
        }
        if r.shadow_color == parent.shadow_color {
            r.shadow_color = None;
        }
        r
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Content {
    Text(String),
    Translatable {
        key: String,
        fallback: Option<String>,
        with: Vec<Component>,
    },
    Keybind(String),
    Score {
        name: String,
        objective: String,
        value: Option<String>,
    },
    Selector {
        pattern: String,
        separator: Option<Box<Component>>,
    },
    Nbt {
        /// "block" | "entity" | "storage"
        kind: NbtSource,
        path: String,
        interpret: bool,
        separator: Option<Box<Component>>,
    },
    /// Object component with sprite contents (`<sprite:atlas:id>`).
    Sprite {
        atlas: Option<String>,
        sprite: String,
    },
    /// A virtual wrapper emitted at depth 0 by modifying tags (gradient/rainbow/
    /// transition/pride). Compaction does NOT flatten it (adventure treats virtual
    /// components as non-text); it serializes like an empty text component.
    Virtual,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NbtSource {
    Block(String),
    Entity(String),
    Storage(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Component {
    pub content: Content,
    pub style: Style,
    pub children: Vec<Component>,
}

impl Component {
    pub fn text(content: impl Into<String>) -> Component {
        Component {
            content: Content::Text(content.into()),
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn empty() -> Component {
        Component::text("")
    }

    pub fn translatable(key: impl Into<String>) -> Component {
        Component {
            content: Content::Translatable {
                key: key.into(),
                fallback: None,
                with: Vec::new(),
            },
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn has_styling(&self) -> bool {
        self.style.is_present()
    }

    /// Plain text of just this node (the Text content if any), used by plain serializer.
    pub fn own_text(&self) -> &str {
        match &self.content {
            Content::Text(s) => s,
            _ => "",
        }
    }

    /// Equivalent to Component.compact() (the default MiniMessage post-processor).
    pub fn compact(&self) -> Component {
        compact(self, None)
    }
}

fn is_text(c: &Component) -> bool {
    matches!(c.content, Content::Text(_))
}

fn text_content(c: &Component) -> Option<&str> {
    match &c.content {
        Content::Text(s) => Some(s.as_str()),
        _ => None,
    }
}

fn join_text(one: &Component, two: &Component) -> Component {
    // Adventure joinText(one, two): two's children, one's style, concatenated content.
    let content = match (&one.content, &two.content) {
        (Content::Text(a), Content::Text(b)) => Content::Text(format!("{a}{b}")),
        _ => one.content.clone(),
    };
    Component {
        content,
        style: one.style.clone(),
        children: two.children.clone(),
    }
}

/// Port of ComponentCompaction.compact (SIMPLIFY_STYLE_FOR_BLANK_COMPONENTS = false).
fn compact(this: &Component, parent_style: Option<&Style>) -> Component {
    let children = &this.children;
    let mut optimized = Component {
        content: this.content.clone(),
        style: this.style.clone(),
        children: Vec::new(),
    };
    if let Some(ps) = parent_style {
        optimized.style = this.style.unmerge(ps);
    }

    if children.is_empty() {
        return optimized;
    }

    // Single empty-text parent: merge its style into the only child and compact that.
    if children.len() == 1 && is_text(&optimized) && text_content(&optimized) == Some("") {
        let child = &children[0];
        let mut new_child = child.clone();
        new_child.style.merge_if_absent(&optimized.style);
        return compact(&new_child, None);
    }

    let mut child_parent_style = optimized.style.clone();
    if let Some(ps) = parent_style {
        child_parent_style.merge_if_absent(ps);
    }

    let mut to_append: Vec<Component> = Vec::new();
    for child in children {
        let c = compact(child, Some(&child_parent_style));
        if c.children.is_empty() && is_text(&c) && text_content(&c) == Some("") {
            continue;
        }
        to_append.push(c);
    }

    // Absorb leading text children whose effective style matches the parent's.
    if is_text(&optimized) {
        while !to_append.is_empty() {
            let child3 = &to_append[0];
            let mut child_style = child3.style.clone();
            child_style.merge_if_absent(&child_parent_style);
            if !is_text(child3) || child_style != child_parent_style {
                break;
            }
            let child3_children = child3.children.clone();
            optimized = join_text(&optimized, child3);
            to_append.remove(0);
            for (idx, cc) in child3_children.into_iter().enumerate() {
                to_append.insert(idx, cc);
            }
        }
    }

    // Merge adjacent text children with equal effective styles.
    let mut i = 0;
    while i + 1 < to_append.len() {
        let merge = {
            let child = &to_append[i];
            let neighbor = &to_append[i + 1];
            if child.children.is_empty() && is_text(child) && is_text(neighbor) {
                let mut cs = child.style.clone();
                cs.merge_if_absent(&child_parent_style);
                let mut ns = neighbor.style.clone();
                ns.merge_if_absent(&child_parent_style);
                cs == ns
            } else {
                false
            }
        };
        if merge {
            let combined = join_text(&to_append[i], &to_append[i + 1]);
            to_append[i] = combined;
            to_append.remove(i + 1);
        } else {
            i += 1;
        }
    }

    optimized.children = to_append;
    optimized
}
