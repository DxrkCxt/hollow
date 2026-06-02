// SPDX-License-Identifier: GPL-3.0-only

use uuid::Uuid;

use crate::constants;
use hollow_protocol::ByteMessage;
use hollow_protocol::packet::{HandshakeIntent, Inbound};
use crate::packets::login::plugin_request::PacketLoginPluginRequest;
use crate::packets::status::ping::PacketStatusPing;
use crate::packets::status::response::PacketStatusResponse;
use hollow_protocol::registry::{State, Version};
use crate::server::log;
use crate::forwarding;
use hollow_protocol::uuid_util as uuid_util;

use super::client_connection::{ClientConnection, red_text};

fn random_positive_int() -> i32 {
    (Uuid::new_v4().as_u128() as i32) & i32::MAX
}

pub fn handle(conn: &ClientConnection, inbound: Inbound) {
    match inbound {
        Inbound::Handshake {
            version,
            host,
            port: _,
            intent,
        } => handle_handshake(conn, version, host, intent),
        Inbound::StatusRequest => handle_status_request(conn),
        Inbound::StatusPing { payload } => {
            conn.shared.send_packet_and_close(&PacketStatusPing { payload });
        }
        Inbound::LoginStart { username, uuid } => handle_login_start(conn, username, uuid),
        Inbound::LoginPluginResponse {
            message_id,
            success,
            data,
        } => handle_login_plugin_response(conn, message_id, success, data),
        Inbound::LoginAcknowledged => conn.on_login_acknowledged_received(),
        Inbound::FinishConfiguration => conn.spawn_player(),
        Inbound::KnownPacks => conn.on_known_packs_received(),
        Inbound::KeepAlive { .. } => {}
        Inbound::PluginMessage { .. } => {}
    }
}

fn handle_handshake(conn: &ClientConnection, version: Version, host: String, intent: HandshakeIntent) {
    conn.update_version(version);
    let config = &conn.server.config;

    match intent {
        HandshakeIntent::Status => {
            conn.update_state(State::Status);
            log::debug(format!(
                "Pinged from {} [{:?}]",
                conn.shared.address(),
                conn.version()
            ));
        }
        HandshakeIntent::Login | HandshakeIntent::Transfer => {
            conn.update_state(State::Login);

            if !conn.version().is_supported() {
                conn.disconnect(red_text("Unsupported client version"));
                return;
            }

            if config.info_forwarding.is_legacy() {
                let split: Vec<&str> = host.split('\0').collect();
                if split.len() == 3 || split.len() == 4 {
                    conn.set_address(split[1].to_string());
                    conn.shared.game_profile.lock().unwrap().uuid = uuid_util::from_string(split[2]);
                } else {
                    conn.disconnect(red_text(
                        "You've enabled player info forwarding. You need to connect with proxy",
                    ));
                }
            } else if config.info_forwarding.is_bungee_guard() {
                match forwarding::check_bungee_guard_handshake(&host, &config.info_forwarding) {
                    Some((addr, uuid)) => {
                        conn.set_address(addr);
                        conn.shared.game_profile.lock().unwrap().uuid = Some(uuid);
                    }
                    None => {
                        conn.disconnect(red_text("Invalid BungeeGuard token or handshake format"));
                    }
                }
            }
        }
        HandshakeIntent::Unknown => conn.disconnect(red_text("Invalid handshake intent!")),
    }
}

fn handle_status_request(conn: &ClientConnection) {
    let config = &conn.server.config;
    let static_protocol = config.ping_data.protocol;

    let protocol = if static_protocol > 0 {
        static_protocol
    } else if config.info_forwarding.is_none() {
        conn.version().protocol_number()
    } else {
        Version::max().protocol_number()
    };

    let response = PacketStatusResponse {
        version_name: hollow_text::legacy::serialize_section(&config.ping_data.version),
        protocol,
        max_players: config.max_players,
        online_players: conn.server.connections.count() as i32,
        description: config.ping_data.description.clone(),
        sample: Vec::new(),
    };
    conn.send_packet(&response);
}

fn handle_login_start(conn: &ClientConnection, username: String, _uuid: Option<Uuid>) {
    let config = &conn.server.config;

    if config.max_players > 0 && conn.server.connections.count() as i32 >= config.max_players {
        conn.disconnect(red_text("Too many players connected"));
        return;
    }

    if config.info_forwarding.is_modern() {
        let login_id = random_positive_int();
        let mut msg = ByteMessage::new();
        msg.write_u8(forwarding::VELOCITY_MAX_SUPPORTED_FORWARDING_VERSION);
        let request = PacketLoginPluginRequest {
            message_id: login_id,
            channel: constants::VELOCITY_INFO_CHANNEL.to_string(),
            data: msg.to_byte_array(),
        };
        *conn.shared.velocity_login_message_id.lock().unwrap() = login_id;
        conn.send_packet(&request);
        return;
    }

    {
        let mut profile = conn.shared.game_profile.lock().unwrap();
        profile.username = Some(username.clone());
        profile.uuid = Some(uuid_util::offline_mode_uuid(&username));
    }
    conn.fire_login_success();
}

fn handle_login_plugin_response(
    conn: &ClientConnection,
    message_id: i32,
    success: bool,
    data: Option<Vec<u8>>,
) {
    let config = &conn.server.config;
    if !(config.info_forwarding.is_modern()
        && message_id == *conn.shared.velocity_login_message_id.lock().unwrap())
    {
        return;
    }

    let data = match (success, data) {
        (true, Some(d)) => d,
        _ => {
            conn.disconnect(red_text("You need to connect with Velocity"));
            return;
        }
    };

    let mut msg = ByteMessage::from_bytes(&data);
    if !forwarding::check_velocity_key_integrity(&config.info_forwarding.secret_key, &mut msg) {
        conn.disconnect(red_text("Can't verify forwarded player info"));
        return;
    }

    let result = (|| -> hollow_protocol::DecodeResult<(i32, String, Uuid, String)> {
        let version = msg.read_var_int()?;
        let address = msg.read_string()?;
        let uuid = msg.read_uuid()?;
        let username = msg.read_string()?;
        Ok((version, address, uuid, username))
    })();

    let (fwd_version, address, uuid, username) = match result {
        Ok(v) => v,
        Err(_) => {
            conn.disconnect(red_text("Can't verify forwarded player info"));
            return;
        }
    };

    let max = forwarding::VELOCITY_MAX_SUPPORTED_FORWARDING_VERSION as i32;
    if fwd_version > max {
        conn.disconnect(red_text(&format!(
            "Unsupported forwarding version {fwd_version}, wanted upto {max}"
        )));
        return;
    }

    conn.set_address(address);
    {
        let mut profile = conn.shared.game_profile.lock().unwrap();
        profile.uuid = Some(uuid);
        profile.username = Some(username);
    }
    conn.fire_login_success();
}
