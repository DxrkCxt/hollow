// SPDX-License-Identifier: GPL-3.0-only

//! MiniMessage deserializer, ported from adventure-text-minimessage.
//!
//! Pipeline mirrors MiniMessageParser: tokenize -> build node tree ->
//! `tree_to_component` (styling tags are `Inserting` empty-text-with-style; gradient
//! and rainbow are `Modifying` and run per-codepoint) -> `compact()` post-processor.
//! Gradient/rainbow color math is ported exactly from GradientTag/RainbowTag.

use super::color::{Color, NamedColor};
use super::component::{
    ClickAction, ClickEvent, Component, Content, Decoration, HoverEvent, NbtSource, Style,
};

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Token {
    Text(String),
    Open {
        name: String,
        args: Vec<String>,
    },
    Close {
        name: String,
    },
}

fn is_known_tag(name: &str) -> bool {
    let base = name.strip_prefix('!').unwrap_or(name);
    if resolve_color_token(base).is_some() {
        return true;
    }
    if base.starts_with('#') {
        return Color::from_hex(base).is_some();
    }
    matches!(
        base,
        "color"
            | "colour"
            | "c"
            | "bold"
            | "b"
            | "italic"
            | "i"
            | "em"
            | "underlined"
            | "u"
            | "strikethrough"
            | "st"
            | "obfuscated"
            | "obf"
            | "reset"
            | "gradient"
            | "rainbow"
            | "newline"
            | "br"
            | "font"
            | "insertion"
            | "insert"
            | "click"
            | "hover"
            | "key"
            | "lang"
            | "translatable"
            | "tr"
            | "score"
            | "selector"
            | "sel"
            | "nbt"
            | "data"
            | "transition"
            | "pride"
            | "shadow"
            | "sprite"
            | "img"
    )
}

/// Reads a `<...>` starting at `start` ('<'); returns (body, end_index_of '>').
fn read_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut body = String::new();
    let mut i = start + 1;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
                body.push(c);
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                    body.push(c);
                } else if c == '>' {
                    if body.is_empty() {
                        return None;
                    }
                    return Some((body, i));
                } else if c == '<' {
                    return None;
                } else {
                    body.push(c);
                }
            }
        }
        i += 1;
    }
    None
}

/// Splits a tag body on ':' while honoring single/double quotes, stripping quotes.
fn split_args(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in body.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                } else if c == ':' {
                    parts.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
            }
        }
    }
    parts.push(cur);
    parts
}

fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() && (chars[i + 1] == '<' || chars[i + 1] == '>') {
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '<' {
            if let Some((body, end)) = read_tag(&chars, i) {
                let is_close = body.starts_with('/');
                let raw_body = if is_close { &body[1..] } else { &body[..] };
                let parts = split_args(raw_body);
                let name = parts[0].trim().to_ascii_lowercase();
                if is_known_tag(&name) {
                    if !text.is_empty() {
                        tokens.push(Token::Text(std::mem::take(&mut text)));
                    }
                    if is_close {
                        tokens.push(Token::Close { name });
                    } else {
                        let args = parts[1..].to_vec();
                        tokens.push(Token::Open { name, args });
                    }
                    i = end + 1;
                    continue;
                }
                // Unknown tag -> literal text.
                let raw: String = chars[i..=end].iter().collect();
                text.push_str(&raw);
                i = end + 1;
                continue;
            }
            text.push('<');
            i += 1;
            continue;
        }
        text.push(c);
        i += 1;
    }
    if !text.is_empty() {
        tokens.push(Token::Text(text));
    }
    tokens
}

// ---------------------------------------------------------------------------
// Node tree
// ---------------------------------------------------------------------------

enum Node {
    Text(String),
    Tag {
        name: String,
        args: Vec<String>,
        children: Vec<Node>,
    },
}

struct Frame {
    name: String,
    args: Vec<String>,
    children: Vec<Node>,
}

fn push_node(stack: &mut [Frame], root: &mut Vec<Node>, node: Node) {
    match stack.last_mut() {
        Some(frame) => frame.children.push(node),
        None => root.push(node),
    }
}

fn build_tree(tokens: Vec<Token>) -> Vec<Node> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Vec<Node> = Vec::new();

    for token in tokens {
        match token {
            Token::Text(s) => push_node(&mut stack, &mut root, Node::Text(s)),
            Token::Open { name, args } => {
                if name == "reset" {
                    // Reset closes every open tag back to the root.
                    while let Some(frame) = stack.pop() {
                        let node = Node::Tag {
                            name: frame.name,
                            args: frame.args,
                            children: frame.children,
                        };
                        push_node(&mut stack, &mut root, node);
                    }
                    continue;
                }
                stack.push(Frame {
                    name,
                    args,
                    children: Vec::new(),
                });
            }
            Token::Close { name } => {
                if let Some(pos) = stack.iter().rposition(|f| f.name == name) {
                    while stack.len() > pos {
                        let frame = stack.pop().unwrap();
                        let node = Node::Tag {
                            name: frame.name,
                            args: frame.args,
                            children: frame.children,
                        };
                        push_node(&mut stack, &mut root, node);
                    }
                } else {
                    // Unmatched close -> literal.
                    push_node(&mut stack, &mut root, Node::Text(format!("</{name}>")));
                }
            }
        }
    }
    while let Some(frame) = stack.pop() {
        let node = Node::Tag {
            name: frame.name,
            args: frame.args,
            children: frame.children,
        };
        push_node(&mut stack, &mut root, node);
    }
    root
}

// ---------------------------------------------------------------------------
// Tag resolution
// ---------------------------------------------------------------------------

enum TagKind {
    Inserting(Component),
    Modifying(Box<dyn ColorChanger>),
}

/// Resolve a named/hex color from a single token (e.g. "red" or "#ff0000").
fn resolve_color_token(token: &str) -> Option<Color> {
    if let Some(stripped) = token.strip_prefix('#') {
        return Color::from_hex(stripped);
    }
    if let Some(named) = NamedColor::from_name(token) {
        return Some(Color::Named(named));
    }
    // Aliases used by MiniMessage.
    match token {
        "grey" => Some(Color::Named(NamedColor::Gray)),
        "dark_grey" => Some(Color::Named(NamedColor::DarkGray)),
        _ => None,
    }
}

fn decoration_for(name: &str) -> Option<Decoration> {
    Some(match name {
        "bold" | "b" => Decoration::Bold,
        "italic" | "i" | "em" => Decoration::Italic,
        "underlined" | "u" => Decoration::Underlined,
        "strikethrough" | "st" => Decoration::Strikethrough,
        "obfuscated" | "obf" => Decoration::Obfuscated,
        _ => return None,
    })
}

fn styling(style: Style) -> TagKind {
    let mut comp = Component::text("");
    comp.style = style;
    TagKind::Inserting(comp)
}

fn resolve_tag(name: &str, args: &[String]) -> Option<TagKind> {
    // Decoration negation: <!bold>
    if let Some(base) = name.strip_prefix('!')
        && let Some(deco) = decoration_for(base)
    {
        let mut style = Style::default();
        style.set_decoration(deco, Some(false));
        return Some(styling(style));
    }

    // <!shadow> -> shadow none (ShadowColor.none() == 0)
    if name == "!shadow" {
        let mut style = Style::default();
        style.shadow_color = Some(0);
        return Some(styling(style));
    }

    // Direct named/hex color: <red>, <#ff0000>
    if let Some(color) = resolve_color_token(name) {
        let mut style = Style::default();
        style.color = Some(color);
        return Some(styling(style));
    }

    match name {
        "color" | "colour" | "c" => {
            let color = args.first().and_then(|a| resolve_color_token(&a.to_ascii_lowercase()))?;
            let mut style = Style::default();
            style.color = Some(color);
            Some(styling(style))
        }
        "bold" | "b" | "italic" | "i" | "em" | "underlined" | "u" | "strikethrough" | "st"
        | "obfuscated" | "obf" => {
            let deco = decoration_for(name).unwrap();
            let value = !matches!(args.first().map(|s| s.as_str()), Some("false") | Some("off"));
            let mut style = Style::default();
            style.set_decoration(deco, Some(value));
            Some(styling(style))
        }
        "newline" | "br" => Some(TagKind::Inserting(Component::text("\n"))),
        "font" => {
            let font = args.first()?.clone();
            let mut style = Style::default();
            style.font = Some(font);
            Some(styling(style))
        }
        "insertion" | "insert" => {
            let insertion = args.first()?.clone();
            let mut style = Style::default();
            style.insertion = Some(insertion);
            Some(styling(style))
        }
        "click" => {
            let action = ClickAction::from_name(&args.first()?.to_ascii_lowercase())?;
            let value = args.get(1).cloned().unwrap_or_default();
            let mut style = Style::default();
            style.click = Some(ClickEvent { action, value });
            Some(styling(style))
        }
        "hover" => resolve_hover(args).map(|h| {
            let mut style = Style::default();
            style.hover = Some(h);
            styling(style)
        }),
        "key" => {
            let key = args.first()?.clone();
            Some(TagKind::Inserting(Component {
                content: Content::Keybind(key),
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        "lang" | "translatable" | "tr" => {
            let key = args.first()?.clone();
            let with: Vec<Component> = args[1..].iter().map(|a| Component::text(a.clone())).collect();
            Some(TagKind::Inserting(Component {
                content: Content::Translatable {
                    key,
                    fallback: None,
                    with,
                },
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        "score" => {
            let nm = args.first()?.clone();
            let objective = args.get(1).cloned().unwrap_or_default();
            Some(TagKind::Inserting(Component {
                content: Content::Score {
                    name: nm,
                    objective,
                    value: None,
                },
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        "selector" | "sel" => {
            let pattern = args.first()?.clone();
            Some(TagKind::Inserting(Component {
                content: Content::Selector {
                    pattern,
                    separator: None,
                },
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        "nbt" | "data" => {
            // <nbt:block|entity|storage:path:interpret?>
            let source = args.first()?.to_ascii_lowercase();
            let path = args.get(1).cloned().unwrap_or_default();
            let id = args.get(2).cloned().unwrap_or_default();
            let kind = match source.as_str() {
                "block" => NbtSource::Block(id),
                "entity" => NbtSource::Entity(id),
                _ => NbtSource::Storage(id),
            };
            Some(TagKind::Inserting(Component {
                content: Content::Nbt {
                    kind,
                    path,
                    interpret: false,
                    separator: None,
                },
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        "gradient" => Some(TagKind::Modifying(Box::new(GradientChanger::new(args)))),
        "rainbow" => Some(TagKind::Modifying(Box::new(RainbowChanger::new(args)))),
        "transition" => {
            let mut style = Style::default();
            style.color = Some(transition_color(args));
            Some(styling(style))
        }
        "pride" => {
            let mut flag = "pride".to_string();
            let mut phase = 0.0f64;
            if let Some(first) = args.first() {
                let value = first.to_ascii_lowercase();
                if is_pride_flag(&value) {
                    flag = value;
                } else if !value.is_empty()
                    && let Ok(p) = value.parse::<f64>()
                {
                    phase = p;
                }
            }
            let colors = pride_flag_colors(&flag);
            Some(TagKind::Modifying(Box::new(GradientChanger::from_colors(
                colors, phase,
            ))))
        }
        "shadow" => {
            let argb = shadow_argb(args)?;
            let mut style = Style::default();
            style.shadow_color = Some(argb);
            Some(styling(style))
        }
        "sprite" | "img" => {
            let first = args.first()?.clone();
            let content = match args.get(1) {
                Some(second) => Content::Sprite {
                    atlas: Some(first),
                    sprite: second.clone(),
                },
                None => Content::Sprite {
                    atlas: None,
                    sprite: first,
                },
            };
            Some(TagKind::Inserting(Component {
                content,
                style: Style::default(),
                children: Vec::new(),
            }))
        }
        _ => None,
    }
}

fn resolve_hover(args: &[String]) -> Option<HoverEvent> {
    let action = args.first()?.to_ascii_lowercase();
    match action.as_str() {
        "show_text" => {
            let text = args.get(1).cloned().unwrap_or_default();
            Some(HoverEvent::ShowText(Box::new(deserialize(&text))))
        }
        "show_item" => Some(HoverEvent::ShowItem {
            id: args.get(1).cloned().unwrap_or_default(),
            count: args.get(2).and_then(|s| s.parse().ok()),
        }),
        "show_entity" => Some(HoverEvent::ShowEntity {
            id_type: args.get(1).cloned().unwrap_or_default(),
            id: args.get(2).cloned().unwrap_or_default(),
            name: args.get(3).map(|s| Box::new(deserialize(s))),
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Modifying tags (gradient / rainbow), ported from AbstractColorChangingTag.
// ---------------------------------------------------------------------------

pub trait ColorChanger {
    fn init(&mut self, size: usize);
    fn advance(&mut self);
    fn color(&self) -> Color;
}

struct GradientChanger {
    colors: Vec<Color>,
    phase: f64,
    negative_phase: bool,
    index: i64,
    multiplier: f64,
    size: usize,
}

impl GradientChanger {
    fn new(args: &[String]) -> GradientChanger {
        let mut colors: Vec<Color> = Vec::new();
        let mut phase = 0.0;
        let n = args.len();
        for (i, arg) in args.iter().enumerate() {
            let lower = arg.to_ascii_lowercase();
            if let Some(c) = resolve_color_token(&lower) {
                colors.push(c);
            } else if i == n - 1
                && let Ok(p) = arg.parse::<f64>()
            {
                phase = p;
            }
        }
        GradientChanger::from_colors(colors, phase)
    }

    /// Shared with the `pride` tag (a gradient over predefined flag colors).
    fn from_colors(mut colors: Vec<Color>, mut phase: f64) -> GradientChanger {
        if colors.is_empty() {
            colors = vec![Color::Rgb(0xFFFFFF), Color::Rgb(0x000000)];
        }
        let negative_phase = phase < 0.0;
        if negative_phase {
            phase += 1.0;
            colors.reverse();
        }
        GradientChanger {
            colors,
            phase,
            negative_phase,
            index: 0,
            multiplier: 1.0,
            size: 0,
        }
    }
}

/// Predefined pride flag colour lists, ported from PrideTag.
fn pride_flag_colors(flag: &str) -> Vec<Color> {
    let rgb = |v: u32| Color::Rgb(v);
    let list: &[u32] = match flag {
        "progress" => &[
            0xFFFFFF, 16756679, 7591918, 6371605, 0, 0xE50000, 16747776, 0xFFEE00, 164129, 19711,
            0x770088,
        ],
        "trans" => &[6017019, 16100281, 0xFFFFFF, 16100281, 6017019],
        "bi" => &[14025328, 10178454, 14504],
        "pan" => &[16718989, 16766720, 1750015],
        "nb" => &[16577585, 0xFCFCFC, 10312146, 0x282828],
        "lesbian" => &[14034944, 16751446, 0xFFFFFF, 13918886, 10748002],
        "ace" => &[0, 0xA4A4A4, 0xFFFFFF, 0x810081],
        "agender" => &[0, 0xBABABA, 0xFFFFFF, 12252292, 0xFFFFFF, 0xBABABA, 0],
        "demisexual" => &[0, 0xFFFFFF, 7209073, 0xD3D3D3],
        "genderqueer" => &[11894749, 0xFFFFFF, 4817438],
        "genderfluid" => &[16676514, 0xFFFFFF, 12522199, 0, 3161278],
        "intersex" => &[16766976, 7930538, 16766976],
        "aro" => &[3909440, 11064442, 0xFFFFFF, 0xABABAB, 0],
        "baker" => &[
            13461247, 16737689, 0xFE0000, 16685312, 0xFFFF01, 39168, 39371, 3473561, 0x990099,
        ],
        "philly" => &[0, 7884567, 0xFE0000, 16616448, 16770304, 1154827, 410803, 12725980],
        "queer" => &[
            0, 10148330, 41960, 11920669, 0xFFFFFF, 16763149, 16541287, 16690889, 0,
        ],
        "gay" => &[495216, 2543274, 10021057, 0xFFFFFF, 8105442, 5261771, 4004472],
        "bigender" => &[12876192, 15509195, 14010344, 0xFFFFFF, 14010344, 10143720, 7111631],
        "demigender" => &[0x7F7F7F, 0xC3C3C3, 16514932, 0xFFFFFF, 16514932, 0xC3C3C3, 0x7F7F7F],
        // default "pride"
        _ => &[0xE50000, 16747776, 0xFFEE00, 164129, 19711, 0x770088],
    };
    list.iter().map(|&v| rgb(v)).collect()
}

fn is_pride_flag(name: &str) -> bool {
    matches!(
        name,
        "pride"
            | "progress"
            | "trans"
            | "bi"
            | "pan"
            | "nb"
            | "lesbian"
            | "ace"
            | "agender"
            | "demisexual"
            | "genderqueer"
            | "genderfluid"
            | "intersex"
            | "aro"
            | "baker"
            | "philly"
            | "queer"
            | "gay"
            | "bigender"
            | "demigender"
    )
}

/// Single interpolated colour for the `<transition>` tag, ported from TransitionTag.
fn transition_color(args: &[String]) -> Color {
    let mut colors: Vec<Color> = Vec::new();
    let mut phase = 0.0f32;
    let n = args.len();
    for (i, arg) in args.iter().enumerate() {
        let lower = arg.to_ascii_lowercase();
        if let Some(c) = resolve_color_token(&lower) {
            colors.push(c);
        } else if i == n - 1
            && let Ok(p) = arg.parse::<f32>()
        {
            phase = p;
        }
    }
    let negative_phase = phase < 0.0;
    if negative_phase {
        phase += 1.0;
        colors.reverse();
    }
    if colors.is_empty() {
        colors = vec![Color::Rgb(0xFFFFFF), Color::Rgb(0x000000)];
    }
    let len = colors.len();
    if len == 1 {
        return colors[0];
    }
    let steps = 1.0 / (len - 1) as f32;
    for color_index in 1..len {
        let val = color_index as f32 * steps;
        if val >= phase {
            let factor = 1.0 + (phase - val) * (len - 1) as f32;
            return if negative_phase {
                lerp(1.0 - factor, colors[color_index], colors[color_index - 1])
            } else {
                lerp(factor, colors[color_index - 1], colors[color_index])
            };
        }
    }
    colors[0]
}

/// Parse a `<shadow:...>` ARGB value, ported from ShadowColorTag.
fn shadow_argb(args: &[String]) -> Option<i32> {
    let arg0 = args.first()?.to_ascii_lowercase();
    if let Some(hex) = arg0.strip_prefix('#') {
        if hex.len() == 8 {
            let rgba = u32::from_str_radix(hex, 16).ok()?;
            let rgb = (rgba >> 8) & 0x00FF_FFFF;
            let alpha = rgba & 0xFF;
            return Some(((alpha << 24) | rgb) as i32);
        }
        return None;
    }
    let color = resolve_color_token(&arg0)?;
    let alpha = args
        .get(1)
        .and_then(|a| a.parse::<f32>().ok())
        .unwrap_or(0.25);
    let a = ((alpha * 255.0) as u32) & 0xFF;
    Some(((a << 24) | (color.value() & 0x00FF_FFFF)) as i32)
}

impl ColorChanger for GradientChanger {
    fn init(&mut self, size: usize) {
        self.size = size;
        self.multiplier = if size == 1 {
            0.0
        } else {
            (self.colors.len() - 1) as f64 / (size - 1) as f64
        };
        self.phase *= (self.colors.len() - 1) as f64;
        self.index = 0;
        let _ = self.negative_phase;
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn color(&self) -> Color {
        let len = self.colors.len() as i64;
        let position = self.index as f64 * self.multiplier + self.phase;
        let low_unclamped = position.floor() as i64;
        let high = (position.ceil() as i64).rem_euclid(len) as usize;
        let low = low_unclamped.rem_euclid(len) as usize;
        let t = position as f32 - low_unclamped as f32;
        lerp(t, self.colors[low], self.colors[high])
    }
}

struct RainbowChanger {
    reversed: bool,
    divided_phase: f64,
    color_index: i64,
    size: usize,
}

impl RainbowChanger {
    fn new(args: &[String]) -> RainbowChanger {
        let mut reversed = false;
        let mut phase = 0i64;
        if let Some(first) = args.first() {
            let mut value = first.as_str();
            if let Some(rest) = value.strip_prefix('!') {
                reversed = true;
                value = rest;
            }
            if !value.is_empty()
                && let Ok(p) = value.parse::<i64>()
            {
                phase = p;
            }
        }
        RainbowChanger {
            reversed,
            divided_phase: phase as f64 / 10.0,
            color_index: 0,
            size: 0,
        }
    }
}

impl ColorChanger for RainbowChanger {
    fn init(&mut self, size: usize) {
        self.size = size;
        if self.reversed {
            self.color_index = size as i64 - 1;
        }
    }

    fn advance(&mut self) {
        if self.reversed {
            self.color_index = if self.color_index == 0 {
                self.size as i64 - 1
            } else {
                self.color_index - 1
            };
        } else {
            self.color_index += 1;
        }
    }

    fn color(&self) -> Color {
        let index = self.color_index as f32;
        let hue = (((index / self.size as f32) as f64 + self.divided_phase) % 1.0) as f32;
        hsv_to_color(hue, 1.0, 1.0)
    }
}

/// Ported from TextColor.lerp.
fn lerp(t: f32, a: Color, b: Color) -> Color {
    let t = t.clamp(0.0, 1.0);
    let chan = |av: u8, bv: u8| -> u32 { (av as f32 + t * (bv as f32 - av as f32)).round() as u32 };
    let r = chan(a.red(), b.red());
    let g = chan(a.green(), b.green());
    let bl = chan(a.blue(), b.blue());
    Color::of_value((r << 16) | (g << 8) | bl)
}

/// HSV (h,s,v in 0..1) to an RGB color, matching adventure's TextColor.color(HSVLike).
fn hsv_to_color(h: f32, s: f32, v: f32) -> Color {
    if s == 0.0 {
        let g = (v * 255.0).round() as u32;
        return Color::of_value((g << 16) | (g << 8) | g);
    }
    let hh = (h - h.floor()) * 6.0;
    let i = hh.floor() as i32;
    let f = hh - hh.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let to = |x: f32| (x * 255.0).round() as u32;
    Color::of_value((to(r) << 16) | (to(g) << 8) | to(b))
}

struct ColorApplier {
    changer: Box<dyn ColorChanger>,
    disable_depth: i32,
}

impl ColorApplier {
    fn skip_for_len(&mut self, s: &str) {
        for _ in s.chars() {
            self.changer.advance();
        }
    }

    fn apply(&mut self, current: &Component, depth: i32) -> Component {
        // emitVirtuals (default for MiniMessage): at depth 0 the modifying tag wraps
        // its output in a virtual component that compaction will not flatten.
        if depth == 0 {
            return Component {
                content: Content::Virtual,
                style: current.style.clone(),
                children: Vec::new(),
            };
        }

        let has_color = current.style.color.is_some();
        if (self.disable_depth != -1 && depth > self.disable_depth) || has_color {
            if self.disable_depth == -1 || depth < self.disable_depth {
                self.disable_depth = depth;
            }
            if let Content::Text(s) = &current.content {
                let s = s.clone();
                self.skip_for_len(&s);
            }
            let mut c = current.clone();
            c.children = Vec::new();
            return c;
        }
        self.disable_depth = -1;

        if let Content::Text(s) = &current.content {
            if !s.is_empty() {
                let mut parent = Component::text("");
                for ch in s.chars() {
                    let mut style = current.style.clone();
                    style.color = Some(self.changer.color());
                    self.changer.advance();
                    parent.children.push(Component {
                        content: Content::Text(ch.to_string()),
                        style,
                        children: Vec::new(),
                    });
                }
                return parent;
            }
            // Empty text: empty().mergeStyle(current)
            let mut e = Component::text("");
            e.style = current.style.clone();
            return e;
        }

        // Non-text component.
        let mut ret = current.clone();
        ret.children = Vec::new();
        if ret.style.color.is_none() {
            ret.style.color = Some(self.changer.color());
        }
        self.changer.advance();
        ret
    }
}

fn handle_modifying(applier: &mut ColorApplier, current: &Component, depth: i32) -> Component {
    let mut new_comp = applier.apply(current, depth);
    for child in &current.children {
        let handled = handle_modifying(applier, child, depth + 1);
        new_comp.children.push(handled);
    }
    new_comp
}

/// Total codepoint count of text in the subtree, plus 1 per non-text leaf
/// (matching adventure's LENGTH_CALCULATOR unknown-mapper of "_").
fn subtree_size(comp: &Component) -> usize {
    let mut n = match &comp.content {
        Content::Text(s) => s.chars().count(),
        _ => {
            if comp.children.is_empty() {
                1
            } else {
                0
            }
        }
    };
    for child in &comp.children {
        n += subtree_size(child);
    }
    n
}

// ---------------------------------------------------------------------------
// Tree -> Component
// ---------------------------------------------------------------------------

fn tree_to_component(node: &Node) -> Component {
    match node {
        Node::Text(s) => Component::text(s.clone()),
        Node::Tag {
            name,
            args,
            children,
        } => {
            let resolved = resolve_tag(name, args);
            match resolved {
                Some(TagKind::Inserting(value)) => {
                    let mut comp = value;
                    for child in children {
                        comp.children.push(tree_to_component(child));
                    }
                    comp
                }
                Some(TagKind::Modifying(changer)) => {
                    let mut comp = Component::text("");
                    for child in children {
                        comp.children.push(tree_to_component(child));
                    }
                    let size = subtree_size(&comp);
                    let mut applier = ColorApplier {
                        changer,
                        disable_depth: -1,
                    };
                    applier.changer.init(size);
                    handle_modifying(&mut applier, &comp, 0)
                }
                None => {
                    // Should not happen (only known tags reach here); emit children.
                    let mut comp = Component::text("");
                    for child in children {
                        comp.children.push(tree_to_component(child));
                    }
                    comp
                }
            }
        }
    }
}

/// Deserialize a MiniMessage string into a (compacted) component.
pub fn deserialize(input: &str) -> Component {
    let nodes = build_tree(tokenize(input));
    let mut root = Component::empty();
    for node in &nodes {
        root.children.push(tree_to_component(node));
    }
    root.compact()
}

// ---------------------------------------------------------------------------
// Serialization (Component -> MiniMessage), used by the legacy branch of parse().
// ---------------------------------------------------------------------------

fn deco_mm_name(d: Decoration) -> &'static str {
    match d {
        Decoration::Bold => "bold",
        Decoration::Italic => "italic",
        Decoration::Underlined => "underlined",
        Decoration::Strikethrough => "strikethrough",
        Decoration::Obfuscated => "obfuscated",
    }
}

fn mm_escape(s: &str) -> String {
    s.replace('<', "\\<").replace('>', "\\>")
}

fn serialize_node(comp: &Component, parent: &Style, out: &mut String) {
    let mut opened: Vec<String> = Vec::new();

    if comp.style.color != parent.color
        && let Some(c) = comp.style.color
    {
        let name = match c {
            Color::Named(n) => n.name().to_string(),
            Color::Rgb(_) => c.hex_string(),
        };
        out.push('<');
        out.push_str(&name);
        out.push('>');
        opened.push(name);
    }
    for d in Decoration::DECORATIONS {
        if comp.style.decoration(d) == Some(true) && parent.decoration(d) != Some(true) {
            let name = deco_mm_name(d);
            out.push('<');
            out.push_str(name);
            out.push('>');
            opened.push(name.to_string());
        }
    }

    if let Content::Text(s) = &comp.content {
        out.push_str(&mm_escape(s));
    }

    let mut effective = comp.style.clone();
    effective.merge_if_absent(parent);
    for child in &comp.children {
        serialize_node(child, &effective, out);
    }

    for name in opened.iter().rev() {
        out.push_str("</");
        out.push_str(name);
        out.push('>');
    }
}

/// Serialize a component to a MiniMessage string (covers colors, decorations, text).
pub fn serialize(comp: &Component) -> String {
    let mut out = String::new();
    serialize_node(comp, &Style::default(), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gson;

    #[test]
    fn plain_text() {
        let c = deserialize("Hello");
        assert_eq!(gson::serialize(&c, gson::MODERN), "\"Hello\"");
    }

    #[test]
    fn named_color() {
        let c = deserialize("<white>Welcome!");
        assert_eq!(
            gson::serialize(&c, gson::MODERN),
            "{\"color\":\"white\",\"text\":\"Welcome!\"}"
        );
    }

    #[test]
    fn bold_short() {
        let c = deserialize("<b>hi");
        assert_eq!(
            gson::serialize(&c, gson::MODERN),
            "{\"bold\":true,\"text\":\"hi\"}"
        );
    }

    #[test]
    fn gradient_two_colors() {
        // Endpoints land exactly on named colours, so they serialize as "blue"/"white"
        // (TextColor.color() maps exact-match values back to named colours).
        let c = deserialize("<gradient:blue:white>AB");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"color\":\"blue\""), "json={json}");
        assert!(json.contains("\"color\":\"white\""), "json={json}");
    }

    #[test]
    fn nested_color_inside_gradient_kept() {
        // An explicit inner color disables gradient for that span.
        let c = deserialize("<gradient:blue:white>A<red>B");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"red\""), "json={json}");
    }

    #[test]
    fn transition_phase_zero_is_first_color() {
        // phase 0 -> first colour = red, which is an exact named match.
        let c = deserialize("<transition:red:blue:0>x");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"color\":\"red\""), "json={json}");
    }

    #[test]
    fn pride_default_flag() {
        let c = deserialize("<pride>AB");
        let json = gson::serialize(&c, gson::MODERN);
        // pride colours are non-named RGB -> UPPERCASE hex
        assert!(json.contains("\"#E50000\""), "json={json}");
        assert!(json.contains("\"#770088\""), "json={json}");
    }

    #[test]
    fn shadow_color_emitted() {
        // #RRGGBBAA -> ARGB integer (0x80ff0000 = -2130771968)
        let c = deserialize("<shadow:#ff000080>hi");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"shadow_color\":-2130771968"), "json={json}");
    }

    #[test]
    fn shadow_none() {
        let c = deserialize("<!shadow>hi");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"shadow_color\":0"), "json={json}");
    }

    #[test]
    fn sprite_object() {
        let c = deserialize("<sprite:minecraft:icon>");
        let json = gson::serialize(&c, gson::MODERN);
        assert!(json.contains("\"sprite\":\"icon\""), "json={json}");
        assert!(json.contains("\"atlas\":\"minecraft\""), "json={json}");
    }

    /// Exact gson output captured from the real adventure 4.26.1 jar (golden), proving
    /// the virtual-wrapper + compaction behaviour is byte-identical to Java.
    #[test]
    fn golden_compaction_matches_java() {
        let cases = [
            (
                "<gradient:blue:white>X",
                "{\"extra\":[{\"color\":\"blue\",\"text\":\"X\"}],\"text\":\"\"}",
            ),
            ("<white>Welcome!", "{\"color\":\"white\",\"text\":\"Welcome!\"}"),
            (
                "<white><bold>Hi",
                "{\"bold\":true,\"color\":\"white\",\"text\":\"Hi\"}",
            ),
            (
                "<bold><gradient:blue:white>AB",
                "{\"bold\":true,\"extra\":[{\"extra\":[{\"color\":\"blue\",\"text\":\"A\"},{\"color\":\"white\",\"text\":\"B\"}],\"text\":\"\"}],\"text\":\"\"}",
            ),
            (
                "<white>Hi<red>Bye",
                "{\"color\":\"white\",\"extra\":[{\"color\":\"red\",\"text\":\"Bye\"}],\"text\":\"Hi\"}",
            ),
            (
                "<gradient:blue:white>AB",
                "{\"extra\":[{\"extra\":[{\"color\":\"blue\",\"text\":\"A\"},{\"color\":\"white\",\"text\":\"B\"}],\"text\":\"\"}],\"text\":\"\"}",
            ),
        ];
        for (input, expected) in cases {
            let got = gson::serialize(&deserialize(input), gson::MODERN);
            assert_eq!(got, expected, "input={input}");
        }
    }
}
