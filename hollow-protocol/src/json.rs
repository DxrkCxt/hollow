// SPDX-License-Identifier: GPL-3.0-only

//! JSON helpers.

/// Strict JSON validity check, equivalent to JsonUtils.isValidJson
/// (Gson's strict TypeAdapter<JsonElement>): a bare word is not valid JSON.
pub fn is_valid_json(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity() {
        assert!(is_valid_json("{\"text\":\"hi\"}"));
        assert!(is_valid_json("\"quoted\""));
        assert!(is_valid_json("123"));
        assert!(!is_valid_json("<gradient:blue:white>Hollow"));
        assert!(!is_valid_json("Hollow"));
    }
}
