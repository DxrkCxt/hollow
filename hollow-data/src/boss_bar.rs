// SPDX-License-Identifier: GPL-3.0-only

use hollow_text::Component;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

impl BossBarColor {
    pub fn index(self) -> i32 {
        match self {
            BossBarColor::Pink => 0,
            BossBarColor::Blue => 1,
            BossBarColor::Red => 2,
            BossBarColor::Green => 3,
            BossBarColor::Yellow => 4,
            BossBarColor::Purple => 5,
            BossBarColor::White => 6,
        }
    }

    pub fn from_name(name: &str) -> Option<BossBarColor> {
        Some(match name.to_ascii_uppercase().as_str() {
            "PINK" => BossBarColor::Pink,
            "BLUE" => BossBarColor::Blue,
            "RED" => BossBarColor::Red,
            "GREEN" => BossBarColor::Green,
            "YELLOW" => BossBarColor::Yellow,
            "PURPLE" => BossBarColor::Purple,
            "WHITE" => BossBarColor::White,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossBarDivision {
    Solid,
    Dashes6,
    Dashes10,
    Dashes12,
    Dashes20,
}

impl BossBarDivision {
    pub fn index(self) -> i32 {
        match self {
            BossBarDivision::Solid => 0,
            BossBarDivision::Dashes6 => 1,
            BossBarDivision::Dashes10 => 2,
            BossBarDivision::Dashes12 => 3,
            BossBarDivision::Dashes20 => 4,
        }
    }

    pub fn from_name(name: &str) -> Option<BossBarDivision> {
        Some(match name.to_ascii_uppercase().as_str() {
            "SOLID" => BossBarDivision::Solid,
            "DASHES_6" => BossBarDivision::Dashes6,
            "DASHES_10" => BossBarDivision::Dashes10,
            "DASHES_12" => BossBarDivision::Dashes12,
            "DASHES_20" => BossBarDivision::Dashes20,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BossBar {
    pub text: Component,
    pub health: f32,
    pub color: BossBarColor,
    pub division: BossBarDivision,
}
