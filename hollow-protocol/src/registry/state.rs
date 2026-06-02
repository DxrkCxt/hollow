// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::sync::OnceLock;

use super::version::Version;

/// Identifies a packet type, used as the registry key in both directions
/// (equivalent to mapping by `Class<?>` in the Java original).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PacketKind {
    Handshake,
    StatusRequest,
    StatusResponse,
    StatusPing,
    LoginStart,
    LoginPluginResponse,
    LoginAcknowledged,
    LoginDisconnect,
    LoginSuccess,
    LoginPluginRequest,
    PluginMessage,
    Disconnect,
    FinishConfiguration,
    KeepAlive,
    KnownPacks,
    UpdateTags,
    RegistryData,
    DeclareCommands,
    /// Play "join game" packet (PacketLogin in the Java sources).
    JoinGame,
    PlayerAbilities,
    PlayerPositionAndLook,
    ChatMessage,
    BossBar,
    PlayerInfo,
    TitleLegacy,
    TitleSetTitle,
    TitleSetSubTitle,
    TitleTimes,
    PlayerListHeader,
    SpawnPosition,
    GameEvent,
    ChunkWithLight,
}

impl PacketKind {
    /// Java class simple name, used for log parity (PacketUtils.toDetailedInfo).
    pub fn class_name(self) -> &'static str {
        use PacketKind::*;
        match self {
            Handshake => "PacketHandshake",
            StatusRequest => "PacketStatusRequest",
            StatusResponse => "PacketStatusResponse",
            StatusPing => "PacketStatusPing",
            LoginStart => "PacketLoginStart",
            LoginPluginResponse => "PacketLoginPluginResponse",
            LoginAcknowledged => "PacketLoginAcknowledged",
            LoginDisconnect => "PacketLoginDisconnect",
            LoginSuccess => "PacketLoginSuccess",
            LoginPluginRequest => "PacketLoginPluginRequest",
            PluginMessage => "PacketPluginMessage",
            Disconnect => "PacketDisconnect",
            FinishConfiguration => "PacketFinishConfiguration",
            KeepAlive => "PacketKeepAlive",
            KnownPacks => "PacketKnownPacks",
            UpdateTags => "PacketUpdateTags",
            RegistryData => "PacketRegistryData",
            DeclareCommands => "PacketDeclareCommands",
            JoinGame => "PacketLogin",
            PlayerAbilities => "PacketPlayerAbilities",
            PlayerPositionAndLook => "PacketPlayerPositionAndLook",
            ChatMessage => "PacketChatMessage",
            BossBar => "PacketBossBar",
            PlayerInfo => "PacketPlayerInfo",
            TitleLegacy => "PacketTitleLegacy",
            TitleSetTitle => "PacketTitleSetTitle",
            TitleSetSubTitle => "PacketTitleSetSubTitle",
            TitleTimes => "PacketTitleTimes",
            PlayerListHeader => "PacketPlayerListHeader",
            SpawnPosition => "PacketSpawnPosition",
            GameEvent => "PacketGameEvent",
            ChunkWithLight => "PacketChunkWithLight",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Handshaking => "HANDSHAKING",
            State::Status => "STATUS",
            State::Login => "LOGIN",
            State::Configuration => "CONFIGURATION",
            State::Play => "PLAY",
        }
    }
}

/// Per-version registry holding both id->kind (for decoding) and kind->id (for encoding),
/// equivalent to Java's PacketRegistry (packetsById / packetIdByClass).
#[derive(Default)]
pub struct PacketRegistry {
    pub id_to_kind: HashMap<i32, PacketKind>,
    pub kind_to_id: HashMap<PacketKind, i32>,
}

impl PacketRegistry {
    pub fn get_kind(&self, id: i32) -> Option<PacketKind> {
        self.id_to_kind.get(&id).copied()
    }

    pub fn get_id(&self, kind: PacketKind) -> Option<i32> {
        self.kind_to_id.get(&kind).copied()
    }

    fn register(&mut self, id: i32, kind: PacketKind) {
        self.id_to_kind.insert(id, kind);
        self.kind_to_id.insert(kind, id);
    }
}

struct Mapping {
    id: i32,
    from: Version,
    to: Version,
}

#[inline]
fn m(id: i32, from: Version, to: Version) -> Mapping {
    Mapping { id, from, to }
}

/// Walk the version range [from, to] following declaration order (mirrors State.getRange).
fn range(mapping: &Mapping) -> Vec<Version> {
    if mapping.to == mapping.from {
        return vec![mapping.from];
    }
    let mut versions = Vec::new();
    let mut curr = mapping.to;
    while curr != mapping.from {
        versions.push(curr);
        curr = curr.prev().expect("range stays within declared versions");
    }
    versions.push(mapping.from);
    versions
}

#[derive(Default)]
pub struct ProtocolMappings {
    map: HashMap<Version, PacketRegistry>,
}

impl ProtocolMappings {
    /// Equivalent to ProtocolMappings.getRegistry: fall back to the min version's registry.
    pub fn get_registry(&self, version: Version) -> Option<&PacketRegistry> {
        self.map
            .get(&version)
            .or_else(|| self.map.get(&Version::min()))
    }

    fn register(&mut self, kind: PacketKind, mappings: &[Mapping]) {
        for mapping in mappings {
            for ver in range(mapping) {
                self.map.entry(ver).or_default().register(mapping.id, kind);
            }
        }
    }
}

struct DirMappings {
    server: ProtocolMappings,
    client: ProtocolMappings,
}

pub struct Registry {
    states: HashMap<State, DirMappings>,
}

impl Registry {
    pub fn server_registry(&self, state: State, version: Version) -> Option<&PacketRegistry> {
        self.states.get(&state)?.server.get_registry(version)
    }

    pub fn client_registry(&self, state: State, version: Version) -> Option<&PacketRegistry> {
        self.states.get(&state)?.client.get_registry(version)
    }

    fn build() -> Registry {
        use PacketKind::*;
        use Version::*;

        let min = Version::min();
        let max = Version::max();
        let mut states: HashMap<State, DirMappings> = HashMap::new();

        // ---- HANDSHAKING ----
        {
            let mut server = ProtocolMappings::default();
            let client = ProtocolMappings::default();
            server.register(Handshake, &[m(0x00, min, max)]);
            states.insert(State::Handshaking, DirMappings { server, client });
        }

        // ---- STATUS ----
        {
            let mut server = ProtocolMappings::default();
            let mut client = ProtocolMappings::default();
            server.register(StatusRequest, &[m(0x00, min, max)]);
            server.register(StatusPing, &[m(0x01, min, max)]);
            client.register(StatusResponse, &[m(0x00, min, max)]);
            client.register(StatusPing, &[m(0x01, min, max)]);
            states.insert(State::Status, DirMappings { server, client });
        }

        // ---- LOGIN ----
        {
            let mut server = ProtocolMappings::default();
            let mut client = ProtocolMappings::default();
            server.register(LoginStart, &[m(0x00, min, max)]);
            server.register(LoginPluginResponse, &[m(0x02, min, max)]);
            server.register(LoginAcknowledged, &[m(0x03, V1_20_2, max)]);
            client.register(LoginDisconnect, &[m(0x00, min, max)]);
            client.register(LoginSuccess, &[m(0x02, min, max)]);
            client.register(LoginPluginRequest, &[m(0x04, min, max)]);
            states.insert(State::Login, DirMappings { server, client });
        }

        // ---- CONFIGURATION ----
        {
            let mut server = ProtocolMappings::default();
            let mut client = ProtocolMappings::default();
            client.register(
                PluginMessage,
                &[m(0x00, V1_20_2, V1_20_3), m(0x01, V1_20_5, max)],
            );
            client.register(
                Disconnect,
                &[m(0x01, V1_20_2, V1_20_3), m(0x02, V1_20_5, max)],
            );
            client.register(
                FinishConfiguration,
                &[m(0x02, V1_20_2, V1_20_3), m(0x03, V1_20_5, max)],
            );
            client.register(
                KeepAlive,
                &[m(0x03, V1_20_2, V1_20_3), m(0x04, V1_20_5, max)],
            );
            client.register(KnownPacks, &[m(0x0E, V1_20_5, max)]);
            client.register(UpdateTags, &[m(0x0D, V1_20_5, max)]);
            client.register(
                RegistryData,
                &[m(0x05, V1_20_2, V1_20_3), m(0x07, V1_20_5, max)],
            );

            server.register(
                PluginMessage,
                &[m(0x01, V1_20_2, V1_20_3), m(0x02, V1_20_2, max)],
            );
            server.register(
                FinishConfiguration,
                &[m(0x02, V1_20_2, V1_20_3), m(0x03, V1_20_5, max)],
            );
            server.register(
                KeepAlive,
                &[m(0x03, V1_20_2, V1_20_3), m(0x04, V1_20_5, max)],
            );
            server.register(KnownPacks, &[m(0x07, V1_20_5, max)]);
            states.insert(State::Configuration, DirMappings { server, client });
        }

        // ---- PLAY ----
        {
            let mut server = ProtocolMappings::default();
            let mut client = ProtocolMappings::default();

            server.register(
                KeepAlive,
                &[
                    m(0x00, V1_7_2, V1_8),
                    m(0x0B, V1_9, V1_11_1),
                    m(0x0C, V1_12, V1_12),
                    m(0x0B, V1_12_1, V1_12_2),
                    m(0x0E, V1_13, V1_13_2),
                    m(0x0F, V1_14, V1_15_2),
                    m(0x10, V1_16, V1_16_4),
                    m(0x0F, V1_17, V1_18_2),
                    m(0x11, V1_19, V1_19),
                    m(0x12, V1_19_1, V1_19_1),
                    m(0x11, V1_19_3, V1_19_3),
                    m(0x12, V1_19_4, V1_20),
                    m(0x14, V1_20_2, V1_20_2),
                    m(0x15, V1_20_3, V1_20_3),
                    m(0x18, V1_20_5, V1_21),
                    m(0x1A, V1_21_2, V1_21_5),
                    m(0x1B, V1_21_6, V1_21_11),
                    m(0x1C, V26_1, max),
                ],
            );
            client.register(
                Disconnect,
                &[
                    m(0x40, V1_7_2, V1_8),
                    m(0x1A, V1_9, V1_12_2),
                    m(0x1B, V1_13, V1_13_2),
                    m(0x1A, V1_14, V1_14_4),
                    m(0x1B, V1_15, V1_15_2),
                    m(0x1A, V1_16, V1_16_1),
                    m(0x19, V1_16_2, V1_16_4),
                    m(0x1A, V1_17, V1_18_2),
                    m(0x17, V1_19, V1_19),
                    m(0x19, V1_19_1, V1_19_1),
                    m(0x17, V1_19_3, V1_19_3),
                    m(0x1A, V1_19_4, V1_20),
                    m(0x1B, V1_20_2, V1_20_2),
                    m(0x15, V1_20_3, V1_20_3),
                    m(0x1D, V1_20_5, V1_21_4),
                    m(0x1C, V1_21_5, V1_21_7),
                    m(0x20, V1_21_9, max),
                ],
            );
            client.register(
                DeclareCommands,
                &[
                    m(0x11, V1_13, V1_14_4),
                    m(0x12, V1_15, V1_15_2),
                    m(0x11, V1_16, V1_16_1),
                    m(0x10, V1_16_2, V1_16_4),
                    m(0x12, V1_17, V1_18_2),
                    m(0x0F, V1_19, V1_19_1),
                    m(0x0E, V1_19_3, V1_19_3),
                    m(0x10, V1_19_4, V1_20),
                    m(0x11, V1_20_2, V1_21_4),
                    m(0x10, V1_21_5, max),
                ],
            );
            client.register(
                JoinGame,
                &[
                    m(0x01, V1_7_2, V1_8),
                    m(0x23, V1_9, V1_12_2),
                    m(0x25, V1_13, V1_14_4),
                    m(0x26, V1_15, V1_15_2),
                    m(0x25, V1_16, V1_16_1),
                    m(0x24, V1_16_2, V1_16_4),
                    m(0x26, V1_17, V1_18_2),
                    m(0x23, V1_19, V1_19),
                    m(0x25, V1_19_1, V1_19_1),
                    m(0x24, V1_19_3, V1_19_3),
                    m(0x28, V1_19_4, V1_20),
                    m(0x29, V1_20_2, V1_20_3),
                    m(0x2B, V1_20_5, V1_21),
                    m(0x2C, V1_21_2, V1_21_4),
                    m(0x2B, V1_21_5, V1_21_7),
                    m(0x30, V1_21_9, V1_21_11),
                    m(0x31, V26_1, max),
                ],
            );
            client.register(
                PluginMessage,
                &[
                    m(0x19, V1_13, V1_13_2),
                    m(0x18, V1_14, V1_14_4),
                    m(0x19, V1_15, V1_15_2),
                    m(0x18, V1_16, V1_16_1),
                    m(0x17, V1_16_2, V1_16_4),
                    m(0x18, V1_17, V1_18_2),
                    m(0x15, V1_19, V1_19),
                    m(0x16, V1_19_1, V1_19_1),
                    m(0x15, V1_19_3, V1_19_3),
                    m(0x17, V1_19_4, V1_20),
                    m(0x18, V1_20_2, V1_20_3),
                    m(0x19, V1_20_5, V1_21_4),
                    m(0x18, V1_21_5, max),
                ],
            );
            client.register(
                PlayerAbilities,
                &[
                    m(0x39, V1_7_2, V1_8),
                    m(0x2B, V1_9, V1_12),
                    m(0x2C, V1_12_1, V1_12_2),
                    m(0x2E, V1_13, V1_13_2),
                    m(0x31, V1_14, V1_14_4),
                    m(0x32, V1_15, V1_15_2),
                    m(0x31, V1_16, V1_16_1),
                    m(0x30, V1_16_2, V1_16_4),
                    m(0x32, V1_17, V1_18_2),
                    m(0x2F, V1_19, V1_19),
                    m(0x31, V1_19_1, V1_19_1),
                    m(0x30, V1_19_3, V1_19_3),
                    m(0x34, V1_19_4, V1_20),
                    m(0x36, V1_20_2, V1_20_3),
                    m(0x38, V1_20_5, V1_21),
                    m(0x3A, V1_21_2, V1_21_4),
                    m(0x39, V1_21_5, V1_21_7),
                    m(0x3E, V1_21_9, V1_21_11),
                    m(0x40, V26_1, max),
                ],
            );
            client.register(
                PlayerPositionAndLook,
                &[
                    m(0x08, V1_7_2, V1_8),
                    m(0x2E, V1_9, V1_12),
                    m(0x2F, V1_12_1, V1_12_2),
                    m(0x32, V1_13, V1_13_2),
                    m(0x35, V1_14, V1_14_4),
                    m(0x36, V1_15, V1_15_2),
                    m(0x35, V1_16, V1_16_1),
                    m(0x34, V1_16_2, V1_16_4),
                    m(0x38, V1_17, V1_18_2),
                    m(0x36, V1_19, V1_19),
                    m(0x39, V1_19_1, V1_19_1),
                    m(0x38, V1_19_3, V1_19_3),
                    m(0x3C, V1_19_4, V1_20),
                    m(0x3E, V1_20_2, V1_20_3),
                    m(0x40, V1_20_5, V1_21),
                    m(0x42, V1_21_2, V1_21_4),
                    m(0x41, V1_21_5, V1_21_7),
                    m(0x46, V1_21_9, V1_21_11),
                    m(0x48, V26_1, max),
                ],
            );
            client.register(
                KeepAlive,
                &[
                    m(0x00, V1_7_2, V1_8),
                    m(0x1F, V1_9, V1_12_2),
                    m(0x21, V1_13, V1_13_2),
                    m(0x20, V1_14, V1_14_4),
                    m(0x21, V1_15, V1_15_2),
                    m(0x20, V1_16, V1_16_1),
                    m(0x1F, V1_16_2, V1_16_4),
                    m(0x21, V1_17, V1_18_2),
                    m(0x1E, V1_19, V1_19),
                    m(0x20, V1_19_1, V1_19_1),
                    m(0x1F, V1_19_3, V1_19_3),
                    m(0x23, V1_19_4, V1_20),
                    m(0x24, V1_20_2, V1_20_3),
                    m(0x26, V1_20_5, V1_21),
                    m(0x27, V1_21_2, V1_21_4),
                    m(0x26, V1_21_5, V1_21_7),
                    m(0x2B, V1_21_9, V1_21_11),
                    m(0x2C, V26_1, max),
                ],
            );
            client.register(
                ChatMessage,
                &[
                    m(0x02, V1_7_2, V1_8),
                    m(0x0F, V1_9, V1_12_2),
                    m(0x0E, V1_13, V1_14_4),
                    m(0x0F, V1_15, V1_15_2),
                    m(0x0E, V1_16, V1_16_4),
                    m(0x0F, V1_17, V1_18_2),
                    m(0x5F, V1_19, V1_19),
                    m(0x62, V1_19_1, V1_19_1),
                    m(0x60, V1_19_3, V1_19_3),
                    m(0x64, V1_19_4, V1_20),
                    m(0x67, V1_20_2, V1_20_2),
                    m(0x69, V1_20_3, V1_20_3),
                    m(0x6C, V1_20_5, V1_21),
                    m(0x73, V1_21_2, V1_21_4),
                    m(0x72, V1_21_5, V1_21_7),
                    m(0x77, V1_21_9, V1_21_11),
                    m(0x79, V26_1, max),
                ],
            );
            client.register(
                BossBar,
                &[
                    m(0x0C, V1_9, V1_14_4),
                    m(0x0D, V1_15, V1_15_2),
                    m(0x0C, V1_16, V1_16_4),
                    m(0x0D, V1_17, V1_18_2),
                    m(0x0A, V1_19, V1_19_3),
                    m(0x0B, V1_19_4, V1_20),
                    m(0x0A, V1_20_2, V1_21_4),
                    m(0x09, V1_21_5, max),
                ],
            );
            client.register(
                PlayerInfo,
                &[
                    m(0x38, V1_7_2, V1_8),
                    m(0x2D, V1_9, V1_12),
                    m(0x2E, V1_12_1, V1_12_2),
                    m(0x30, V1_13, V1_13_2),
                    m(0x33, V1_14, V1_14_4),
                    m(0x34, V1_15, V1_15_2),
                    m(0x33, V1_16, V1_16_1),
                    m(0x32, V1_16_2, V1_16_4),
                    m(0x36, V1_17, V1_18_2),
                    m(0x34, V1_19, V1_19),
                    m(0x37, V1_19_1, V1_19_1),
                    m(0x36, V1_19_3, V1_19_3),
                    m(0x3A, V1_19_4, V1_20),
                    m(0x3C, V1_20_2, V1_20_3),
                    m(0x3E, V1_20_5, V1_21),
                    m(0x40, V1_21_2, V1_21_4),
                    m(0x3F, V1_21_5, V1_21_7),
                    m(0x44, V1_21_9, V1_21_11),
                    m(0x46, V26_1, max),
                ],
            );
            client.register(
                TitleLegacy,
                &[
                    m(0x45, V1_8, V1_11_1),
                    m(0x47, V1_12, V1_12),
                    m(0x48, V1_12_1, V1_12_2),
                    m(0x4B, V1_13, V1_13_2),
                    m(0x4F, V1_14, V1_14_4),
                    m(0x50, V1_15, V1_15_2),
                    m(0x4F, V1_16, V1_16_4),
                ],
            );
            client.register(
                TitleSetTitle,
                &[
                    m(0x59, V1_17, V1_17_1),
                    m(0x5A, V1_18, V1_19),
                    m(0x5D, V1_19_1, V1_19_1),
                    m(0x5B, V1_19_3, V1_19_3),
                    m(0x5F, V1_19_4, V1_20),
                    m(0x61, V1_20_2, V1_20_2),
                    m(0x63, V1_20_3, V1_20_3),
                    m(0x65, V1_20_5, V1_21),
                    m(0x6C, V1_21_2, V1_21_4),
                    m(0x6B, V1_21_5, V1_21_7),
                    m(0x70, V1_21_9, V1_21_11),
                    m(0x72, V26_1, max),
                ],
            );
            client.register(
                TitleSetSubTitle,
                &[
                    m(0x57, V1_17, V1_17_1),
                    m(0x58, V1_18, V1_19),
                    m(0x5B, V1_19_1, V1_19_1),
                    m(0x59, V1_19_3, V1_19_3),
                    m(0x5D, V1_19_4, V1_20),
                    m(0x5F, V1_20_2, V1_20_2),
                    m(0x61, V1_20_3, V1_20_3),
                    m(0x63, V1_20_5, V1_21),
                    m(0x6A, V1_21_2, V1_21_4),
                    m(0x69, V1_21_5, V1_21_7),
                    m(0x6E, V1_21_9, V1_21_11),
                    m(0x70, V26_1, max),
                ],
            );
            client.register(
                TitleTimes,
                &[
                    m(0x5A, V1_17, V1_17_1),
                    m(0x5B, V1_18, V1_19),
                    m(0x5E, V1_19_1, V1_19_1),
                    m(0x5C, V1_19_3, V1_19_3),
                    m(0x60, V1_19_4, V1_20),
                    m(0x62, V1_20_2, V1_20_2),
                    m(0x64, V1_20_3, V1_20_3),
                    m(0x66, V1_20_5, V1_21),
                    m(0x6D, V1_21_2, V1_21_4),
                    m(0x6C, V1_21_5, V1_21_7),
                    m(0x71, V1_21_9, V1_21_11),
                    m(0x73, V26_1, max),
                ],
            );
            client.register(
                PlayerListHeader,
                &[
                    m(0x47, V1_8, V1_8),
                    m(0x48, V1_9, V1_9_2),
                    m(0x47, V1_9_4, V1_11_1),
                    m(0x49, V1_12, V1_12),
                    m(0x4A, V1_12_1, V1_12_2),
                    m(0x4E, V1_13, V1_13_2),
                    m(0x53, V1_14, V1_14_4),
                    m(0x54, V1_15, V1_15_2),
                    m(0x53, V1_16, V1_16_4),
                    m(0x5E, V1_17, V1_17_1),
                    m(0x5F, V1_18, V1_18_2),
                    m(0x60, V1_19, V1_19),
                    m(0x63, V1_19_1, V1_19_1),
                    m(0x61, V1_19_3, V1_19_3),
                    m(0x65, V1_19_4, V1_20),
                    m(0x68, V1_20_2, V1_20_2),
                    m(0x6A, V1_20_3, V1_20_3),
                    m(0x6D, V1_20_5, V1_21),
                    m(0x74, V1_21_2, V1_21_4),
                    m(0x73, V1_21_5, V1_21_7),
                    m(0x78, V1_21_9, V1_21_11),
                    m(0x7A, V26_1, max),
                ],
            );
            client.register(
                SpawnPosition,
                &[
                    m(0x4C, V1_19_3, V1_19_3),
                    m(0x50, V1_19_4, V1_20),
                    m(0x52, V1_20_2, V1_20_2),
                    m(0x54, V1_20_3, V1_20_3),
                    m(0x56, V1_20_5, V1_21),
                    m(0x5B, V1_21_2, V1_21_4),
                    m(0x5A, V1_21_5, V1_21_7),
                    m(0x5F, V1_21_9, V1_21_11),
                    m(0x61, V26_1, max),
                ],
            );
            client.register(
                GameEvent,
                &[
                    m(0x20, V1_20_3, V1_20_3),
                    m(0x22, V1_20_5, V1_21),
                    m(0x23, V1_21_2, V1_21_4),
                    m(0x22, V1_21_5, V1_21_7),
                    m(0x26, V1_21_9, max),
                ],
            );
            client.register(
                ChunkWithLight,
                &[
                    m(0x25, V1_20_3, V1_20_3),
                    m(0x27, V1_20_5, V1_21),
                    m(0x28, V1_21_2, V1_21_4),
                    m(0x27, V1_21_5, V1_21_7),
                    m(0x2C, V1_21_9, V1_21_11),
                    m(0x2D, V26_1, max),
                ],
            );

            states.insert(State::Play, DirMappings { server, client });
        }

        Registry { states }
    }
}

/// Global, lazily-built packet registry (equivalent to the static State enum init).
pub fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(Registry::build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_id() {
        let reg = registry();
        let r = reg.server_registry(State::Handshaking, Version::V1_8).unwrap();
        assert_eq!(r.get_kind(0x00), Some(PacketKind::Handshake));
    }

    #[test]
    fn login_success_client_id() {
        let reg = registry();
        let r = reg.client_registry(State::Login, Version::V1_8).unwrap();
        assert_eq!(r.get_id(PacketKind::LoginSuccess), Some(0x02));
        assert_eq!(r.get_id(PacketKind::LoginDisconnect), Some(0x00));
        assert_eq!(r.get_id(PacketKind::LoginPluginRequest), Some(0x04));
    }

    #[test]
    fn play_keepalive_client_ids() {
        let reg = registry();
        // 1.8: 0x00
        let r = reg.client_registry(State::Play, Version::V1_8).unwrap();
        assert_eq!(r.get_id(PacketKind::KeepAlive), Some(0x00));
        // 1.20.2: 0x24
        let r = reg.client_registry(State::Play, Version::V1_20_2).unwrap();
        assert_eq!(r.get_id(PacketKind::KeepAlive), Some(0x24));
        // 26.1: 0x2C
        let r = reg.client_registry(State::Play, Version::V26_1).unwrap();
        assert_eq!(r.get_id(PacketKind::KeepAlive), Some(0x2C));
    }

    #[test]
    fn join_game_ids() {
        let reg = registry();
        // 1.20.5/1.20.6 share protocol 766 -> id 0x2B
        let r = reg.client_registry(State::Play, Version::V1_20_5).unwrap();
        assert_eq!(r.get_id(PacketKind::JoinGame), Some(0x2B));
    }

    #[test]
    fn config_server_registration_order_matches_java() {
        let reg = registry();
        // On 1.20.2/1.20.3 serverbound: 0x01 = PluginMessage, and 0x02 is shadowed by
        // FinishConfiguration (registered after PluginMessage's 0x02 mapping) — exactly
        // mirroring Java's last-registered-wins in packetsById. This is the protocol-correct
        // result (1.20.2 serverbound: plugin message = 0x01, finish config = 0x02).
        let r = reg
            .server_registry(State::Configuration, Version::V1_20_2)
            .unwrap();
        assert_eq!(r.get_kind(0x01), Some(PacketKind::PluginMessage));
        assert_eq!(r.get_kind(0x02), Some(PacketKind::FinishConfiguration));

        // On 1.20.5+: plugin message = 0x02, finish config = 0x03.
        let r = reg
            .server_registry(State::Configuration, Version::V1_20_5)
            .unwrap();
        assert_eq!(r.get_kind(0x02), Some(PacketKind::PluginMessage));
        assert_eq!(r.get_kind(0x03), Some(PacketKind::FinishConfiguration));
    }
}
