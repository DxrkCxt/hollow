// SPDX-License-Identifier: GPL-3.0-only

//! Legacy `&`/`§` serializer, ported from LegacyComponentSerializerImpl.
//!
//! `serialize_section` reproduces `legacySection()` (used by `toLegacyString`,
//! e.g. the ping version field and the F3 brand). `deserialize` reproduces the
//! `&`-character + hex serializer used by the legacy branch of `ComponentUtils.parse`.

use super::color::{Color, NamedColor};
use super::component::{Component, Content, Decoration, Style};

pub const SECTION_CHAR: char = '\u{00A7}';
pub const AMPERSAND_CHAR: char = '&';

fn color_char(c: NamedColor) -> char {
    match c {
        NamedColor::Black => '0',
        NamedColor::DarkBlue => '1',
        NamedColor::DarkGreen => '2',
        NamedColor::DarkAqua => '3',
        NamedColor::DarkRed => '4',
        NamedColor::DarkPurple => '5',
        NamedColor::Gold => '6',
        NamedColor::Gray => '7',
        NamedColor::DarkGray => '8',
        NamedColor::Blue => '9',
        NamedColor::Green => 'a',
        NamedColor::Aqua => 'b',
        NamedColor::Red => 'c',
        NamedColor::LightPurple => 'd',
        NamedColor::Yellow => 'e',
        NamedColor::White => 'f',
    }
}

fn char_color(ch: char) -> Option<NamedColor> {
    Some(match ch.to_ascii_lowercase() {
        '0' => NamedColor::Black,
        '1' => NamedColor::DarkBlue,
        '2' => NamedColor::DarkGreen,
        '3' => NamedColor::DarkAqua,
        '4' => NamedColor::DarkRed,
        '5' => NamedColor::DarkPurple,
        '6' => NamedColor::Gold,
        '7' => NamedColor::Gray,
        '8' => NamedColor::DarkGray,
        '9' => NamedColor::Blue,
        'a' => NamedColor::Green,
        'b' => NamedColor::Aqua,
        'c' => NamedColor::Red,
        'd' => NamedColor::LightPurple,
        'e' => NamedColor::Yellow,
        'f' => NamedColor::White,
        _ => return None,
    })
}

fn deco_char(d: Decoration) -> char {
    match d {
        Decoration::Obfuscated => 'k',
        Decoration::Bold => 'l',
        Decoration::Strikethrough => 'm',
        Decoration::Underlined => 'n',
        Decoration::Italic => 'o',
    }
}

fn char_deco(ch: char) -> Option<Decoration> {
    Some(match ch.to_ascii_lowercase() {
        'k' => Decoration::Obfuscated,
        'l' => Decoration::Bold,
        'm' => Decoration::Strikethrough,
        'n' => Decoration::Underlined,
        'o' => Decoration::Italic,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Serialization (section codes), ported from the Cereal/StyleState flattener.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Color(Color),
    Deco(Decoration),
    Reset,
}

#[derive(Clone, Copy, Default)]
struct DecoSet([bool; 5]);

impl DecoSet {
    fn idx(d: Decoration) -> usize {
        match d {
            Decoration::Obfuscated => 0,
            Decoration::Bold => 1,
            Decoration::Strikethrough => 2,
            Decoration::Underlined => 3,
            Decoration::Italic => 4,
        }
    }
    fn contains(&self, d: Decoration) -> bool {
        self.0[Self::idx(d)]
    }
    /// Adds, returning true if it was newly added.
    fn add(&mut self, d: Decoration) -> bool {
        let i = Self::idx(d);
        let was = self.0[i];
        self.0[i] = true;
        !was
    }
    /// Removes, returning true if it had been present.
    fn remove(&mut self, d: Decoration) -> bool {
        let i = Self::idx(d);
        let was = self.0[i];
        self.0[i] = false;
        was
    }
    fn contains_all(&self, other: &DecoSet) -> bool {
        (0..5).all(|i| !other.0[i] || self.0[i])
    }
    fn iter(&self) -> impl Iterator<Item = Decoration> + '_ {
        Decoration::DECORATIONS
            .into_iter()
            .filter(|&d| self.contains(d))
    }
}

#[derive(Clone, Default)]
struct StyleState {
    color: Option<Color>,
    decorations: DecoSet,
    needs_reset: bool,
}

impl StyleState {
    fn apply(&mut self, style: &Style) {
        if let Some(c) = style.color {
            self.color = Some(c);
        }
        for d in Decoration::DECORATIONS {
            match style.decoration(d) {
                Some(true) => {
                    self.decorations.add(d);
                }
                Some(false) => {
                    // Not collapsible into a match-arm guard: doing so would leave
                    // `Some(false)` uncovered when `remove` returns false.
                    #[allow(clippy::collapsible_match)]
                    if self.decorations.remove(d) {
                        self.needs_reset = true;
                    }
                }
                None => {}
            }
        }
    }
}

struct Cereal {
    sb: String,
    character: char,
    written: StyleState,
    last_written: Option<Format>,
    stack: Vec<StyleState>,
}

impl Cereal {
    fn new(character: char) -> Cereal {
        Cereal {
            sb: String::new(),
            character,
            written: StyleState::default(),
            last_written: None,
            stack: Vec::new(),
        }
    }

    fn push_style(&mut self, style: &Style) {
        let mut state = match self.stack.last() {
            Some(parent) => StyleState {
                color: parent.color,
                decorations: parent.decorations,
                needs_reset: false,
            },
            None => StyleState::default(),
        };
        state.apply(style);
        self.stack.push(state);
    }

    fn pop_style(&mut self) {
        self.stack.pop();
    }

    fn component(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.apply_format();
        self.sb.push_str(text);
    }

    fn apply_format(&mut self) {
        let top = self.stack.last().copied_state();
        let color_changed = top.color != self.written.color;
        if top.needs_reset {
            if !color_changed {
                self.append(Format::Reset);
            }
            if let Some(s) = self.stack.last_mut() {
                s.needs_reset = false;
            }
        }
        if color_changed || self.last_written == Some(Format::Reset) {
            self.apply_full_format(&top);
            return;
        }
        if !top.decorations.contains_all(&self.written.decorations) {
            self.apply_full_format(&top);
            return;
        }
        for d in top.decorations.iter() {
            if self.written.decorations.add(d) {
                self.append(Format::Deco(d));
            }
        }
    }

    fn apply_full_format(&mut self, top: &StyleState) {
        match top.color {
            Some(c) => self.append(Format::Color(c)),
            None => self.append(Format::Reset),
        }
        self.written.color = top.color;
        for d in top.decorations.iter() {
            self.append(Format::Deco(d));
        }
        self.written.decorations = top.decorations;
    }

    fn append(&mut self, format: Format) {
        if self.last_written != Some(format) {
            match to_code(format) {
                Some(code) => {
                    self.sb.push(self.character);
                    self.sb.push(code);
                }
                None => return,
            }
        }
        self.last_written = Some(format);
    }
}

trait CopiedState {
    fn copied_state(self) -> StyleState;
}
impl CopiedState for Option<&StyleState> {
    fn copied_state(self) -> StyleState {
        self.cloned().unwrap_or_default()
    }
}

fn to_code(format: Format) -> Option<char> {
    match format {
        // hexColours is false for legacySection(): downsample to nearest named.
        Format::Color(c) => Some(color_char(c.nearest_named())),
        Format::Deco(d) => Some(deco_char(d)),
        Format::Reset => Some('r'),
    }
}

fn flatten(component: &Component, cereal: &mut Cereal) {
    cereal.push_style(&component.style);
    cereal.component(own_text(component));
    for child in &component.children {
        flatten(child, cereal);
    }
    cereal.pop_style();
}

fn own_text(component: &Component) -> &str {
    match &component.content {
        Content::Text(t) => t,
        Content::Translatable { key, .. } => key,
        Content::Keybind(k) => k,
        Content::Selector { pattern, .. } => pattern,
        _ => "",
    }
}

/// Equivalent to `LegacyComponentSerializer.legacySection().serialize(component)`.
pub fn serialize_section(component: &Component) -> String {
    let mut cereal = Cereal::new(SECTION_CHAR);
    flatten(component, &mut cereal);
    cereal.sb
}

// ---------------------------------------------------------------------------
// Deserialization (`&` character + `&#rrggbb` hex), for ComponentUtils.parse.
// ---------------------------------------------------------------------------

/// Deserialize legacy text into a component. Mirrors the `&`-character serializer
/// with hex colours enabled (`&#rrggbb`). Color codes reset active decorations.
pub fn deserialize(input: &str) -> Component {
    let chars: Vec<char> = input.chars().collect();
    let mut runs: Vec<Component> = Vec::new();
    let mut current = String::new();
    let mut style = Style::default();

    let flush = |runs: &mut Vec<Component>, current: &mut String, style: &Style| {
        if !current.is_empty() {
            let mut c = Component::text(std::mem::take(current));
            c.style = style.clone();
            runs.push(c);
        }
    };

    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == AMPERSAND_CHAR && i + 1 < chars.len() {
            let code = chars[i + 1];
            // Hex color: &#rrggbb
            if code == '#' && i + 7 < chars.len() {
                let hex: String = chars[i + 2..i + 8].iter().collect();
                if let Some(color) = Color::from_hex(&hex) {
                    flush(&mut runs, &mut current, &style);
                    style = Style::default();
                    style.color = Some(color);
                    i += 8;
                    continue;
                }
            }
            if let Some(named) = char_color(code) {
                flush(&mut runs, &mut current, &style);
                style = Style::default();
                style.color = Some(Color::Named(named));
                i += 2;
                continue;
            }
            if let Some(deco) = char_deco(code) {
                flush(&mut runs, &mut current, &style);
                style.set_decoration(deco, Some(true));
                i += 2;
                continue;
            }
            if code.eq_ignore_ascii_case(&'r') {
                flush(&mut runs, &mut current, &style);
                style = Style::default();
                i += 2;
                continue;
            }
        }
        current.push(ch);
        i += 1;
    }
    flush(&mut runs, &mut current, &style);

    if runs.len() == 1 {
        return runs.pop().unwrap();
    }
    let mut root = Component::empty();
    root.children = runs;
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::NamedColor;

    #[test]
    fn plain_text_no_codes() {
        assert_eq!(serialize_section(&Component::text("hello")), "hello");
    }

    #[test]
    fn named_color_then_bold() {
        let mut c = Component::text("hi");
        c.style.color = Some(Color::Named(NamedColor::Red));
        c.style.bold = Some(true);
        // color 'c' then bold 'l'
        assert_eq!(serialize_section(&c), "\u{A7}c\u{A7}lhi");
    }

    #[test]
    fn rgb_downsamples_to_named_code() {
        let mut c = Component::text("x");
        c.style.color = Some(Color::Rgb(0x5555FF)); // -> blue '9'
        assert_eq!(serialize_section(&c), "\u{A7}9x");
    }

    #[test]
    fn deserialize_named_and_text() {
        let c = deserialize("&cRed&r plain");
        // first run colored red, then reset plain
        assert_eq!(crate::plain::serialize(&c), "Red plain");
    }
}
