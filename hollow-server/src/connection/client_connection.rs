// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use hollow_protocol::packet::PacketOut;
use crate::packets::login::disconnect::PacketLoginDisconnect;
use crate::packets::play::disconnect::PacketDisconnect;
use crate::packets::play::keep_alive::PacketKeepAlive;
use hollow_protocol::registry::{State, Version, registry};
use hollow_protocol::{ByteMessage, PacketSnapshot, byte_message};
use crate::server::LimboServer;
use crate::server::log;
use hollow_text::{self, Color, Component, NamedColor};

use super::game_profile::GameProfile;

/// Outbound message: bytes to write, or an empty Vec meaning "flush and close".
pub type Outbound = Vec<u8>;

pub fn red_text(message: &str) -> Component {
    let mut c = Component::text(message);
    c.style.color = Some(Color::Named(NamedColor::Red));
    c
}

/// Frame a packet: varint(length) + varint(id) + body. Returns empty if the packet
/// has no id for this (state, version) — matching the Java encoder's warn-and-skip.
pub fn encode_framed(packet: &dyn PacketOut, state: State, version: Version) -> Vec<u8> {
    let reg = match registry().client_registry(state, version) {
        Some(r) => r,
        None => return Vec::new(),
    };
    let id = match reg.get_id(packet.kind()) {
        Some(i) => i,
        None => {
            log::warning(format!(
                "Undefined packet class: {} [{:?}|{}]",
                packet.kind().class_name(),
                version,
                state.name()
            ));
            return Vec::new();
        }
    };
    // Build the body once (id varint + packet payload), then frame it directly into the
    // outbound Vec: one body buffer, one output allocation, one copy of the body — instead
    // of the three full-payload copies a second ByteMessage + two to_byte_array()s incur.
    let mut body = ByteMessage::new();
    body.write_var_int(id);
    packet.encode(&mut body, version);
    let payload = body.as_read_slice();

    let mut out = Vec::with_capacity(5 + payload.len());
    byte_message::put_var_int(&mut out, payload.len() as i32);
    out.extend_from_slice(payload);
    out
}

fn random_long() -> i64 {
    Uuid::new_v4().as_u64_pair().0 as i64
}

pub struct ConnShared {
    pub id: u64,
    pub sender: UnboundedSender<Outbound>,
    pub encode_state: Mutex<State>,
    pub decode_state: Mutex<State>,
    pub version: Mutex<Version>,
    pub address: Mutex<String>,
    pub game_profile: Mutex<GameProfile>,
    pub velocity_login_message_id: Mutex<i32>,
    pub active: AtomicBool,
}

impl ConnShared {
    pub fn is_connected(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Logical state (equals decoder state; the Netty `state` field).
    pub fn state(&self) -> State {
        *self.decode_state.lock().unwrap()
    }

    pub fn encode_state(&self) -> State {
        *self.encode_state.lock().unwrap()
    }

    pub fn version(&self) -> Version {
        *self.version.lock().unwrap()
    }

    pub fn uuid(&self) -> Option<Uuid> {
        self.game_profile.lock().unwrap().uuid
    }

    pub fn username(&self) -> Option<String> {
        self.game_profile.lock().unwrap().username.clone()
    }

    pub fn address(&self) -> String {
        self.address.lock().unwrap().clone()
    }

    pub fn send_packet(&self, packet: &dyn PacketOut) {
        if self.is_connected() {
            let bytes = encode_framed(packet, self.encode_state(), self.version());
            if !bytes.is_empty() {
                let _ = self.sender.send(bytes);
            }
        }
    }

    pub fn send_packet_and_close(&self, packet: &dyn PacketOut) {
        if self.is_connected() {
            let bytes = encode_framed(packet, self.encode_state(), self.version());
            if !bytes.is_empty() {
                let _ = self.sender.send(bytes);
            }
            let _ = self.sender.send(Vec::new());
            self.active.store(false, Ordering::Relaxed);
        }
    }

    pub fn send_keep_alive(&self) {
        if self.state() == State::Play {
            let ka = PacketKeepAlive {
                id: random_long(),
            };
            self.send_packet(&ka);
        }
    }
}

#[derive(Clone)]
pub struct ClientConnection {
    pub shared: Arc<ConnShared>,
    pub server: Arc<LimboServer>,
}

impl ClientConnection {
    pub fn version(&self) -> Version {
        self.shared.version()
    }

    pub fn state(&self) -> State {
        self.shared.state()
    }

    pub fn send_packet(&self, packet: &dyn PacketOut) {
        self.shared.send_packet(packet);
    }

    pub fn write_packet(&self, packet: &dyn PacketOut) {
        // In the tokio writer-task model, write == send (ordering preserved by the channel).
        self.shared.send_packet(packet);
    }

    fn write_packets(&self, packets: &[PacketSnapshot]) {
        for p in packets {
            self.write_packet(p);
        }
    }

    pub fn update_state(&self, state: State) {
        *self.shared.encode_state.lock().unwrap() = state;
        *self.shared.decode_state.lock().unwrap() = state;
    }

    pub fn update_encoder_state(&self, state: State) {
        *self.shared.encode_state.lock().unwrap() = state;
    }

    pub fn update_version(&self, version: Version) {
        *self.shared.version.lock().unwrap() = version;
    }

    pub fn set_address(&self, host: String) {
        *self.shared.address.lock().unwrap() = host;
    }

    pub fn fire_login_success(&self) {
        let config = &self.server.config;
        if config.info_forwarding.is_modern()
            && *self.shared.velocity_login_message_id.lock().unwrap() == -1
        {
            self.disconnect(red_text("You need to connect with Velocity"));
            return;
        }

        self.send_packet(&self.server.snapshots.packet_login_success);

        self.server.connections.add_connection(self);

        if self.version().more_or_equal(Version::V1_20_2) {
            self.update_encoder_state(State::Configuration);
            return;
        }

        self.spawn_player();
    }

    pub fn spawn_player(&self) {
        self.update_state(State::Play);
        let s = &self.server.snapshots;
        let config = &self.server.config;
        let version = self.version();

        self.write_packet(&s.packet_join_game);
        self.write_packet(&s.packet_player_abilities);

        if version.less(Version::V1_9) {
            self.write_packet(&s.packet_player_pos_and_look_legacy);
        } else {
            self.write_packet(&s.packet_player_pos_and_look);
        }

        if version.more_or_equal(Version::V1_19_3) {
            self.write_packet(&s.packet_spawn_position);
        }

        if config.use_player_list || version == Version::V1_16_4 {
            self.write_packet(&s.packet_player_info);
        }

        if version.more_or_equal(Version::V1_13) {
            self.write_packet(&s.packet_declare_commands);
            if let Some(pm) = &s.packet_plugin_message {
                self.write_packet(pm);
            }
        }

        if let Some(bb) = &s.packet_boss_bar
            && version.more_or_equal(Version::V1_9)
        {
            self.write_packet(bb);
        }

        if let Some(jm) = &s.packet_join_message {
            self.write_packet(jm);
        }

        if s.packet_title_title.is_some() && version.more_or_equal(Version::V1_8) {
            self.write_title();
        }

        if let Some(hf) = &s.packet_header_and_footer
            && version.more_or_equal(Version::V1_8)
        {
            self.write_packet(hf);
        }

        if version.more_or_equal(Version::V1_20_3) {
            self.write_packet(&s.packet_start_waiting_chunks);
            self.write_packets(&s.packets_chunks);
        }

        self.shared.send_keep_alive();
    }

    pub fn on_login_acknowledged_received(&self) {
        self.update_state(State::Configuration);
        let s = &self.server.snapshots;

        if let Some(pm) = &s.packet_plugin_message {
            self.write_packet(pm);
        }

        if self.version().more_or_equal(Version::V1_20_5) {
            self.send_packet(&s.packet_known_packs);
            return;
        }

        self.write_packet(&s.packet_registry_data);
        self.send_packet(&s.packet_finish_configuration);
    }

    pub fn on_known_packs_received(&self) {
        let s = &self.server.snapshots;
        if let Some(registry_data) = s.packets_registry_data.get(&self.version()) {
            self.write_packets(registry_data);
        }
        self.write_packet(&s.packet_update_tags);
        self.send_packet(&s.packet_finish_configuration);
    }

    pub fn write_title(&self) {
        let s = &self.server.snapshots;
        if self.version().more_or_equal(Version::V1_17) {
            if let (Some(t), Some(st), Some(ti)) = (
                &s.packet_title_title,
                &s.packet_title_subtitle,
                &s.packet_title_times,
            ) {
                self.write_packet(t);
                self.write_packet(st);
                self.write_packet(ti);
            }
        } else if let (Some(t), Some(st), Some(ti)) = (
            &s.packet_title_legacy_title,
            &s.packet_title_legacy_subtitle,
            &s.packet_title_legacy_times,
        ) {
            self.write_packet(t);
            self.write_packet(st);
            self.write_packet(ti);
        }
    }

    pub fn disconnect(&self, reason: Component) {
        if !self.shared.is_connected() {
            return;
        }

        let name = self.shared.username().unwrap_or_else(|| self.shared.address());
        log::debug(format!("{} kicked: {}", name, hollow_text::plain::serialize(&reason)));

        let state = self.state();
        if !(state == State::Login || state == State::Configuration || state == State::Play) {
            self.shared.active.store(false, Ordering::Relaxed);
            let _ = self.shared.sender.send(Vec::new());
            return;
        }

        if state == State::Login {
            let packet = PacketLoginDisconnect { reason };
            self.shared.send_packet_and_close(&packet);
        } else {
            let packet = PacketDisconnect { reason };
            self.shared.send_packet_and_close(&packet);
        }
    }
}
