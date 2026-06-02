// SPDX-License-Identifier: GPL-3.0-only

//!
//! The Java original serializes the `Response` POJO (including its embedded
//! `Component description`) with `gsonComponentSerializer.serializer().toJson(response)`.
//! That produces a single JSON string written via `msg.writeString(...)`.
//!
//! We replicate this by building a serde_json object tree that matches the Gson
//! output structure, then writing the serialized string.

use serde_json::{Map, Value};
use uuid::Uuid;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_text::{self, Component};

/// A player entry in the `players.sample` array.
#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub name: String,
    pub unique_id: Uuid,
}

/// The outer response object.
#[derive(Clone, Debug)]
pub struct PacketStatusResponse {
    /// Protocol version name (e.g. "1.21").
    pub version_name: String,
    /// Protocol number.
    pub protocol: i32,
    /// Maximum players advertised.
    pub max_players: i32,
    /// Online player count.
    pub online_players: i32,
    /// Description (MOTD) component.
    pub description: Component,
    /// Optional sample player list (may be empty).
    pub sample: Vec<PlayerInfo>,
}

impl Default for PacketStatusResponse {
    fn default() -> Self {
        PacketStatusResponse {
            version_name: String::new(),
            protocol: 0,
            max_players: 0,
            online_players: 0,
            description: Component::empty(),
            sample: Vec::new(),
        }
    }
}

impl PacketOut for PacketStatusResponse {
    fn encode(&self, buf: &mut ByteMessage, version: Version) {
        let profile = hollow_text::gson::profile_for(version);
        let description_value = hollow_text::gson::to_value(&self.description, profile);

        // Build version object: {"name": "...", "protocol": N}
        let mut version_obj = Map::new();
        version_obj.insert("name".into(), Value::String(self.version_name.clone()));
        version_obj.insert("protocol".into(), Value::Number(self.protocol.into()));

        // Build players.sample array
        let sample_arr: Vec<Value> = self
            .sample
            .iter()
            .map(|p| {
                let mut obj = Map::new();
                obj.insert("name".into(), Value::String(p.name.clone()));
                obj.insert("id".into(), Value::String(p.unique_id.to_string()));
                Value::Object(obj)
            })
            .collect();

        // Build players object: {"max": N, "online": N, "sample": [...]}
        let mut players_obj = Map::new();
        players_obj.insert("max".into(), Value::Number(self.max_players.into()));
        players_obj.insert("online".into(), Value::Number(self.online_players.into()));
        players_obj.insert("sample".into(), Value::Array(sample_arr));

        // Build top-level response object
        let mut root = Map::new();
        root.insert("version".into(), Value::Object(version_obj));
        root.insert("players".into(), Value::Object(players_obj));
        root.insert("description".into(), description_value);

        let json = Value::Object(root).to_string();
        buf.write_string(&json);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::StatusResponse
    }
}
