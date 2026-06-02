// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use uuid::Uuid;

use crate::configuration::LimboConfig;
use crate::constants;
use crate::packets::configuration::finish::PacketFinishConfiguration;
use crate::packets::configuration::known_packs::{KnownPack, PacketKnownPacks};
use crate::packets::configuration::registry_data::PacketRegistryData;
use crate::packets::configuration::update_tags::PacketUpdateTags;
use crate::packets::login::success::PacketLoginSuccess;
use crate::packets::play::boss_bar::PacketBossBar;
use crate::packets::play::chat_message::{PacketChatMessage, PositionLegacy};
use crate::packets::play::chunk_with_light::PacketChunkWithLight;
use crate::packets::play::declare_commands::PacketDeclareCommands;
use crate::packets::play::game_event::PacketGameEvent;
use crate::packets::play::login::PacketLogin;
use crate::packets::play::player_abilities::PacketPlayerAbilities;
use crate::packets::play::player_info::PacketPlayerInfo;
use crate::packets::play::player_list_header::PacketPlayerListHeader;
use crate::packets::play::player_position_and_look::PacketPlayerPositionAndLook;
use crate::packets::play::plugin_message::PacketPluginMessage;
use crate::packets::play::spawn_position::PacketSpawnPosition;
use crate::packets::play::title_legacy::{PacketTitleLegacy, TitleAction};
use crate::packets::play::title_set_subtitle::PacketTitleSetSubTitle;
use crate::packets::play::title_set_title::PacketTitleSetTitle;
use crate::packets::play::title_times::PacketTitleTimes;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};
use hollow_protocol::{ByteMessage, PacketSnapshot};
use hollow_text::legacy;
use hollow_protocol::uuid_util as uuid_util;
use hollow_world::DimensionRegistry;

pub struct PacketSnapshots {
    pub packet_login_success: PacketSnapshot,
    pub packet_join_game: PacketSnapshot,
    pub packet_spawn_position: PacketSnapshot,
    pub packet_player_abilities: PacketSnapshot,
    pub packet_player_info: PacketSnapshot,
    pub packet_declare_commands: PacketSnapshot,
    pub packet_player_pos_and_look_legacy: PacketSnapshot,
    pub packet_player_pos_and_look: PacketSnapshot,
    pub packet_plugin_message: Option<PacketSnapshot>,
    pub packet_join_message: Option<PacketSnapshot>,
    pub packet_boss_bar: Option<PacketSnapshot>,
    pub packet_header_and_footer: Option<PacketSnapshot>,
    pub packet_title_title: Option<PacketSnapshot>,
    pub packet_title_subtitle: Option<PacketSnapshot>,
    pub packet_title_times: Option<PacketSnapshot>,
    pub packet_title_legacy_title: Option<PacketSnapshot>,
    pub packet_title_legacy_subtitle: Option<PacketSnapshot>,
    pub packet_title_legacy_times: Option<PacketSnapshot>,
    pub packet_registry_data: PacketSnapshot,
    pub packets_registry_data: HashMap<Version, Vec<PacketSnapshot>>,
    pub packet_known_packs: PacketSnapshot,
    pub packet_update_tags: PacketSnapshot,
    pub packet_finish_configuration: PacketSnapshot,
    pub packets_chunks: Vec<PacketSnapshot>,
    pub packet_start_waiting_chunks: PacketSnapshot,
}

impl PacketSnapshots {
    pub fn init(config: &LimboConfig, dimension_registry: &DimensionRegistry) -> PacketSnapshots {
        let player_list_name: String = config.player_list_username.chars().take(16).collect();
        let uuid = uuid_util::offline_mode_uuid(&player_list_name);

        let login_success = PacketLoginSuccess {
            username: player_list_name.clone(),
            uuid,
        };

        let versioned_dimension = config
            .dimension_type
            .create_versioned_dimension(dimension_registry);

        let mut join_game = PacketLogin::with_dimension(versioned_dimension.clone());
        join_game.entity_id = 0;
        join_game.enable_respawn_screen = true;
        join_game.flat = false;
        join_game.game_mode = config.game_mode;
        join_game.secure_profile = config.secure_profile;
        join_game.hardcore = false;
        join_game.max_players = config.max_players;
        join_game.previous_game_mode = -1;
        join_game.reduced_debug_info = true;
        join_game.debug = false;
        join_game.view_distance = 0;
        join_game.seed = 0;

        let mut player_abilities = PacketPlayerAbilities::default();
        player_abilities.flying_speed = 0.0;
        player_abilities.flying = true;
        player_abilities.field_of_view = 0.1;

        let teleport_id = (Uuid::new_v4().as_u128() as i32) & i32::MAX;

        let pos_and_look_legacy = PacketPlayerPositionAndLook {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            teleport_id,
        };
        let pos_and_look = PacketPlayerPositionAndLook {
            x: 0.0,
            y: 400.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            teleport_id,
        };

        let spawn_position = PacketSpawnPosition {
            dimension_key: versioned_dimension.key().clone(),
            x: 0,
            y: 400,
            z: 0,
            yaw: 0.0,
            pitch: 0.0,
        };

        let declare_commands = PacketDeclareCommands {
            commands: Vec::new(),
        };

        let player_info = PacketPlayerInfo {
            game_mode: config.game_mode,
            username: player_list_name.clone(),
            uuid,
        };

        let packet_plugin_message = if config.use_brand_name {
            let mut bm = ByteMessage::new();
            bm.write_string(&legacy::serialize_section(&config.brand_name));
            let pm = PacketPluginMessage {
                channel: constants::BRAND_CHANNEL.to_string(),
                data: bm.to_byte_array(),
            };
            Some(PacketSnapshot::of(&pm))
        } else {
            None
        };

        let packet_header_and_footer = if config.use_header_and_footer {
            let header = PacketPlayerListHeader {
                header: config.player_list_header.clone(),
                footer: config.player_list_footer.clone(),
            };
            Some(PacketSnapshot::of(&header))
        } else {
            None
        };

        let packet_join_message = if config.use_join_message {
            let chat = PacketChatMessage {
                message: config.join_message.clone(),
                position: PositionLegacy::SystemMessage,
                sender: Uuid::new_v4(),
            };
            Some(PacketSnapshot::of(&chat))
        } else {
            None
        };

        let packet_boss_bar = if config.use_boss_bar {
            let boss_bar = PacketBossBar {
                uuid: Uuid::new_v4(),
                boss_bar: config.boss_bar.clone().expect("boss bar enabled"),
                flags: 0,
            };
            Some(PacketSnapshot::of(&boss_bar))
        } else {
            None
        };

        let (
            title_title,
            title_subtitle,
            title_times,
            title_legacy_title,
            title_legacy_subtitle,
            title_legacy_times,
        ) = if config.use_title {
            let title = config.title.clone().expect("title enabled");
            let packet_title = PacketTitleSetTitle {
                title: title.title.clone(),
            };
            let packet_subtitle = PacketTitleSetSubTitle {
                subtitle: title.subtitle.clone(),
            };
            let packet_times = PacketTitleTimes {
                fade_in: title.fade_in,
                stay: title.stay,
                fade_out: title.fade_out,
            };

            let make_legacy = |action: TitleAction| PacketTitleLegacy {
                action,
                title: PacketTitleSetTitle {
                    title: title.title.clone(),
                },
                subtitle: PacketTitleSetSubTitle {
                    subtitle: title.subtitle.clone(),
                },
                times: PacketTitleTimes {
                    fade_in: title.fade_in,
                    stay: title.stay,
                    fade_out: title.fade_out,
                },
            };

            (
                Some(PacketSnapshot::of(&packet_title)),
                Some(PacketSnapshot::of(&packet_subtitle)),
                Some(PacketSnapshot::of(&packet_times)),
                Some(PacketSnapshot::of(&make_legacy(TitleAction::SetTitle))),
                Some(PacketSnapshot::of(&make_legacy(TitleAction::SetSubtitle))),
                Some(PacketSnapshot::of(&make_legacy(
                    TitleAction::SetTimesAndDisplay,
                ))),
            )
        } else {
            (None, None, None, None, None, None)
        };

        // Known packs: per-version (the pack version string is the MC version display name).
        let packet_known_packs = PacketSnapshot::build(PacketKind::KnownPacks, Version::VALUES, |v, buf| {
            let p = PacketKnownPacks {
                known_packs: vec![KnownPack {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: v.display_name().to_string(),
                }],
            };
            p.encode(buf, v);
        });

        // Update tags: per-version.
        let packet_update_tags = PacketSnapshot::build(PacketKind::UpdateTags, Version::VALUES, |v, buf| {
            let p = PacketUpdateTags {
                tags: dimension_registry.create_update_tags(v),
            };
            p.encode(buf, v);
        });

        // Registry data (1.20.2-1.20.3): writes the whole 1.20 codec.
        let codec_1_20 = dimension_registry.codec_1_20().clone();
        let registry_data = PacketRegistryData {
            metadata_writer: Box::new(move |buf, version| {
                buf.write_compound_tag(&codec_1_20, version);
            }),
        };
        let packet_registry_data = PacketSnapshot::of(&registry_data);

        // Per-version registry data (>= 1.20.5).
        let mut packets_registry_data: HashMap<Version, Vec<PacketSnapshot>> = HashMap::new();
        for (version, writers) in dimension_registry.create_per_version_registries() {
            let mut snapshots = Vec::new();
            for writer in writers {
                let packet = PacketRegistryData {
                    metadata_writer: writer,
                };
                snapshots.push(PacketSnapshot::of_version(&packet, version));
            }
            packets_registry_data.insert(version, snapshots);
        }

        let packet_finish_configuration = PacketSnapshot::of(&PacketFinishConfiguration);

        let mut game_event = PacketGameEvent::default();
        game_event.event_type = 13; // Waiting for chunks
        game_event.value = 0.0;
        let packet_start_waiting_chunks = PacketSnapshot::of(&game_event);

        let mut packets_chunks = Vec::new();
        let edge = 1;
        for chunk_x in (0 - edge)..=edge {
            for chunk_z in (0 - edge)..=edge {
                let chunk = PacketChunkWithLight::new(chunk_x, chunk_z, versioned_dimension.clone());
                packets_chunks.push(PacketSnapshot::of(&chunk));
            }
        }

        PacketSnapshots {
            packet_login_success: PacketSnapshot::of(&login_success),
            packet_join_game: PacketSnapshot::of(&join_game),
            packet_spawn_position: PacketSnapshot::of(&spawn_position),
            packet_player_abilities: PacketSnapshot::of(&player_abilities),
            packet_player_info: PacketSnapshot::of(&player_info),
            packet_declare_commands: PacketSnapshot::of(&declare_commands),
            packet_player_pos_and_look_legacy: PacketSnapshot::of(&pos_and_look_legacy),
            packet_player_pos_and_look: PacketSnapshot::of(&pos_and_look),
            packet_plugin_message,
            packet_join_message,
            packet_boss_bar,
            packet_header_and_footer,
            packet_title_title: title_title,
            packet_title_subtitle: title_subtitle,
            packet_title_times: title_times,
            packet_title_legacy_title: title_legacy_title,
            packet_title_legacy_subtitle: title_legacy_subtitle,
            packet_title_legacy_times: title_legacy_times,
            packet_registry_data,
            packets_registry_data,
            packet_known_packs,
            packet_update_tags,
            packet_finish_configuration,
            packets_chunks,
            packet_start_waiting_chunks,
        }
    }
}
