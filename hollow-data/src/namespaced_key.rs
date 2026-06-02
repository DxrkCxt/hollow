// SPDX-License-Identifier: GPL-3.0-only

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NamespacedKey {
    pub namespace: String,
    pub key: String,
}

impl NamespacedKey {
    pub const MINECRAFT_NAMESPACE: &'static str = "minecraft";

    pub fn new(namespace: impl Into<String>, key: impl Into<String>) -> NamespacedKey {
        NamespacedKey {
            namespace: namespace.into(),
            key: key.into(),
        }
    }

    pub fn minecraft(key: &str) -> NamespacedKey {
        NamespacedKey {
            namespace: Self::MINECRAFT_NAMESPACE.to_string(),
            key: key.to_ascii_lowercase(),
        }
    }
}

impl std::fmt::Display for NamespacedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.key)
    }
}
