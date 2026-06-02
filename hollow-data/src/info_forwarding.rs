// SPDX-License-Identifier: GPL-3.0-only

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForwardingType {
    None,
    Legacy,
    Modern,
    BungeeGuard,
}

#[derive(Clone, Debug)]
pub struct InfoForwarding {
    pub forwarding_type: ForwardingType,
    pub secret_key: Vec<u8>,
    pub tokens: Vec<String>,
}

impl Default for InfoForwarding {
    fn default() -> Self {
        InfoForwarding {
            forwarding_type: ForwardingType::None,
            secret_key: Vec::new(),
            tokens: Vec::new(),
        }
    }
}

impl InfoForwarding {
    pub fn has_token(&self, token: Option<&str>) -> bool {
        match token {
            Some(t) => self.tokens.iter().any(|x| x == t),
            None => false,
        }
    }

    pub fn is_none(&self) -> bool {
        self.forwarding_type == ForwardingType::None
    }
    pub fn is_legacy(&self) -> bool {
        self.forwarding_type == ForwardingType::Legacy
    }
    pub fn is_modern(&self) -> bool {
        self.forwarding_type == ForwardingType::Modern
    }
    pub fn is_bungee_guard(&self) -> bool {
        self.forwarding_type == ForwardingType::BungeeGuard
    }
}
