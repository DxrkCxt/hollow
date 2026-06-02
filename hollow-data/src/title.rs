// SPDX-License-Identifier: GPL-3.0-only

use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct Title {
    pub title: Component,
    pub subtitle: Component,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}
