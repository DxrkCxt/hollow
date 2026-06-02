// SPDX-License-Identifier: GPL-3.0-only

use hollow_text::Component;

#[derive(Clone, Debug)]
pub struct PingData {
    pub version: Component,
    pub description: Component,
    pub protocol: i32,
}
