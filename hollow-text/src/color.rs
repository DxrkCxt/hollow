// SPDX-License-Identifier: GPL-3.0-only

//! Colors: named colors, RGB colors, and pre-1.16 HSV downsampling.
//!
//! Ported from net.kyori.adventure.text.format.{NamedTextColor, TextColor, TextColorImpl}
//! and net.kyori.adventure.util.HSVLike, so downsampling matches Java byte-for-byte.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NamedColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
}

impl NamedColor {
    /// In the same order as NamedTextColor.VALUES (relied upon by `nearest`).
    pub const VALUES: [NamedColor; 16] = [
        NamedColor::Black,
        NamedColor::DarkBlue,
        NamedColor::DarkGreen,
        NamedColor::DarkAqua,
        NamedColor::DarkRed,
        NamedColor::DarkPurple,
        NamedColor::Gold,
        NamedColor::Gray,
        NamedColor::DarkGray,
        NamedColor::Blue,
        NamedColor::Green,
        NamedColor::Aqua,
        NamedColor::Red,
        NamedColor::LightPurple,
        NamedColor::Yellow,
        NamedColor::White,
    ];

    pub fn name(self) -> &'static str {
        match self {
            NamedColor::Black => "black",
            NamedColor::DarkBlue => "dark_blue",
            NamedColor::DarkGreen => "dark_green",
            NamedColor::DarkAqua => "dark_aqua",
            NamedColor::DarkRed => "dark_red",
            NamedColor::DarkPurple => "dark_purple",
            NamedColor::Gold => "gold",
            NamedColor::Gray => "gray",
            NamedColor::DarkGray => "dark_gray",
            NamedColor::Blue => "blue",
            NamedColor::Green => "green",
            NamedColor::Aqua => "aqua",
            NamedColor::Red => "red",
            NamedColor::LightPurple => "light_purple",
            NamedColor::Yellow => "yellow",
            NamedColor::White => "white",
        }
    }

    pub fn value(self) -> u32 {
        match self {
            NamedColor::Black => 0x000000,
            NamedColor::DarkBlue => 0x0000AA,
            NamedColor::DarkGreen => 0x00AA00,
            NamedColor::DarkAqua => 0x00AAAA,
            NamedColor::DarkRed => 0xAA0000,
            NamedColor::DarkPurple => 0xAA00AA,
            NamedColor::Gold => 0xFFAA00,
            NamedColor::Gray => 0xAAAAAA,
            NamedColor::DarkGray => 0x555555,
            NamedColor::Blue => 0x5555FF,
            NamedColor::Green => 0x55FF55,
            NamedColor::Aqua => 0x55FFFF,
            NamedColor::Red => 0xFF5555,
            NamedColor::LightPurple => 0xFF55FF,
            NamedColor::Yellow => 0xFFFF55,
            NamedColor::White => 0xFFFFFF,
        }
    }

    pub fn from_name(name: &str) -> Option<NamedColor> {
        NamedColor::VALUES.into_iter().find(|c| c.name() == name)
    }

    /// Exact named color for a value, equivalent to NamedTextColor.namedColor(int).
    pub fn exact(value: u32) -> Option<NamedColor> {
        NamedColor::VALUES.into_iter().find(|c| c.value() == value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Named(NamedColor),
    Rgb(u32),
}

impl Color {
    pub fn value(self) -> u32 {
        match self {
            Color::Named(n) => n.value(),
            Color::Rgb(v) => v & 0xFF_FFFF,
        }
    }

    pub fn red(self) -> u8 {
        ((self.value() >> 16) & 0xFF) as u8
    }

    pub fn green(self) -> u8 {
        ((self.value() >> 8) & 0xFF) as u8
    }

    pub fn blue(self) -> u8 {
        (self.value() & 0xFF) as u8
    }

    /// Lowercase `#rrggbb`, equivalent to TextColor.asHexString().
    pub fn hex_string(self) -> String {
        format!("#{:06x}", self.value())
    }

    /// Nearest named color (HSV distance), equivalent to NamedTextColor.nearestTo().
    pub fn nearest_named(self) -> NamedColor {
        match self {
            Color::Named(n) => n,
            Color::Rgb(v) => nearest(v),
        }
    }

    /// Equivalent to adventure's `TextColor.color(value)`: returns a named color when
    /// the value exactly matches one, else an RGB color.
    pub fn of_value(value: u32) -> Color {
        let v = value & 0xFF_FFFF;
        match NamedColor::exact(v) {
            Some(n) => Color::Named(n),
            None => Color::Rgb(v),
        }
    }

    /// Construct from a hex string like `#rrggbb` or `rrggbb`.
    pub fn from_hex(s: &str) -> Option<Color> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 {
            return None;
        }
        u32::from_str_radix(hex, 16).ok().map(Color::Rgb)
    }
}

/// (h, s, v) with h,s,v in 0..1, ported from HSVLike.fromRGB.
fn hsv_from_rgb(value: u32) -> (f32, f32, f32) {
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    let min = r.min(g.min(b));
    let max = r.max(g.max(b));
    let delta = max - min;
    let s = if max != 0.0 { delta / max } else { 0.0 };
    if s == 0.0 {
        return (0.0, s, max);
    }
    let mut h = if r == max {
        (g - b) / delta
    } else if g == max {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    (h / 360.0, s, max)
}

/// Ported from TextColorImpl.distance.
fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let hue_distance = 3.0 * f32::min((a.0 - b.0).abs(), 1.0 - (a.0 - b.0).abs());
    let saturation_diff = a.1 - b.1;
    let value_diff = a.2 - b.2;
    hue_distance * hue_distance + saturation_diff * saturation_diff + value_diff * value_diff
}

/// Ported from TextColor.nearestColorTo over NamedTextColor.VALUES.
fn nearest(value: u32) -> NamedColor {
    let any = hsv_from_rgb(value);
    let mut matched_distance = f32::MAX;
    let mut matched = NamedColor::VALUES[0];
    for potential in NamedColor::VALUES {
        let d = distance(any, hsv_from_rgb(potential.value()));
        if d < matched_distance {
            matched = potential;
            matched_distance = d;
        }
        if d == 0.0 {
            break;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_string_lowercase() {
        assert_eq!(Color::Rgb(0xFF55FF).hex_string(), "#ff55ff");
        assert_eq!(Color::Named(NamedColor::Black).hex_string(), "#000000");
    }

    #[test]
    fn nearest_named_exacts() {
        // Exact named values map to themselves.
        for c in NamedColor::VALUES {
            assert_eq!(Color::Rgb(c.value()).nearest_named(), c, "{}", c.name());
        }
    }

    #[test]
    fn from_name_roundtrip() {
        assert_eq!(NamedColor::from_name("dark_blue"), Some(NamedColor::DarkBlue));
        assert_eq!(NamedColor::from_name("white"), Some(NamedColor::White));
        assert_eq!(NamedColor::from_name("nope"), None);
    }
}
