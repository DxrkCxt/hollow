// SPDX-License-Identifier: GPL-3.0-only

//! Loads and validates `settings.yml`.
//!
//! Parsing is done by reading `settings.yml` into a `serde_norway::Value` tree
//! and then post-processing every field exactly as the corresponding Java
//! TypeSerializer would do.  The file-copy-on-missing logic mirrors
//! `LimboConfig.getReader()`.

use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use hollow_data::{
    BossBar, BossBarColor, BossBarDivision, ForwardingType, InfoForwarding, PingData, Title,
};
use hollow_data::TransportType;
use hollow_text::Component;
use hollow_world::DimensionType;

// ---------------------------------------------------------------------------
// The embedded default settings file (mirrors getReader() fallback copy).
// ---------------------------------------------------------------------------
const DEFAULT_SETTINGS: &str = include_str!("../../resources/settings.yml");

// ---------------------------------------------------------------------------
// Public config struct
// ---------------------------------------------------------------------------

/// Port of `the upstream project`.
///
/// All fields are `pub` so the rest of the application can read them directly,
/// matching the `@Getter`-annotated Java original.
pub struct LimboConfig {
    pub address: SocketAddr,
    pub max_players: i32,
    pub ping_data: PingData,
    pub dimension_type: DimensionType,
    pub game_mode: i32,
    pub secure_profile: bool,
    pub use_brand_name: bool,
    pub use_join_message: bool,
    pub use_boss_bar: bool,
    pub use_title: bool,
    pub use_player_list: bool,
    pub use_header_and_footer: bool,
    pub brand_name: Component,
    pub join_message: Component,
    pub boss_bar: Option<BossBar>,
    pub title: Option<Title>,
    pub player_list_username: String,
    pub player_list_header: Component,
    pub player_list_footer: Component,
    pub info_forwarding: InfoForwarding,
    pub read_timeout: i64,
    pub debug_level: i32,
    pub log_players_ip: bool,
    pub transport_type: TransportType,
    pub boss_group_size: i32,
    pub worker_group_size: i32,
    pub use_traffic_limits: bool,
    pub max_packet_size: i32,
    pub interval: f64,
    pub max_packet_rate: f64,
    pub max_packet_bytes_rate: f64,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_norway::Error),
    Semantic(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error loading config: {e}"),
            ConfigError::Parse(e) => write!(f, "YAML parse error: {e}"),
            ConfigError::Semantic(s) => write!(f, "Config error: {s}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<serde_norway::Error> for ConfigError {
    fn from(e: serde_norway::Error) -> Self {
        ConfigError::Parse(e)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

impl LimboConfig {
    /// Load the configuration from `<root>/settings.yml` (creating the default if absent).
    pub fn load(root: &Path) -> Result<LimboConfig, ConfigError> {
        Self::load_from(&root.join("settings.yml"), &[])
    }

    /// Load configuration from an explicit settings file, then apply `overlays`.
    ///
    /// The file's parent directory is the root for `@file` secret/token references. If
    /// the file does not exist the embedded default is written first (mirroring
    /// `LimboConfig.getReader()`). Overlays are applied to the parsed YAML *before* the
    /// typed config is built, so they pass through the same parsing/validation as the
    /// file (components, forwarding secrets, numeric ranges, …).
    pub fn load_from(config_path: &Path, overlays: &[Overlay]) -> Result<LimboConfig, ConfigError> {
        Self::load_inner(config_path, overlays, true)
    }

    /// Like [`load_from`] but never writes a default file; a missing file falls back to
    /// the embedded defaults. Used by `--healthcheck`, which must not mutate the volume.
    pub fn load_readonly(
        config_path: &Path,
        overlays: &[Overlay],
    ) -> Result<LimboConfig, ConfigError> {
        Self::load_inner(config_path, overlays, false)
    }

    fn load_inner(
        config_path: &Path,
        overlays: &[Overlay],
        create_if_missing: bool,
    ) -> Result<LimboConfig, ConfigError> {
        let text = if config_path.exists() {
            fs::read_to_string(config_path)?
        } else {
            if create_if_missing {
                fs::write(config_path, DEFAULT_SETTINGS)?;
            }
            DEFAULT_SETTINGS.to_string()
        };

        let mut conf: serde_norway::Value = serde_norway::from_str(&text)?;
        for overlay in overlays {
            overlay.apply(&mut conf);
        }

        let root = config_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parse_config(&conf, root)
    }
}

/// A single configuration override: a `settings.yml` key path and the value to set
/// there. Built from CLI flags and `HOLLOW_*` environment variables and applied to the
/// parsed YAML before the typed config is built — so overrides reuse all of the file's
/// parsing and validation.
#[derive(Debug, Clone)]
pub struct Overlay {
    path: Vec<String>,
    value: serde_norway::Value,
}

impl Overlay {
    fn raw(path: &[&str], value: serde_norway::Value) -> Overlay {
        Overlay {
            path: path.iter().map(|s| s.to_string()).collect(),
            value,
        }
    }

    /// String-valued override at `path`.
    pub fn str(path: &[&str], value: impl Into<String>) -> Overlay {
        Overlay::raw(path, serde_norway::Value::String(value.into()))
    }
    /// Integer-valued override.
    pub fn int(path: &[&str], value: i64) -> Overlay {
        Overlay::raw(path, serde_norway::Value::Number(value.into()))
    }
    /// Float-valued override.
    pub fn float(path: &[&str], value: f64) -> Overlay {
        Overlay::raw(path, serde_norway::Value::Number(value.into()))
    }
    /// Boolean-valued override.
    pub fn bool(path: &[&str], value: bool) -> Overlay {
        Overlay::raw(path, serde_norway::Value::Bool(value))
    }

    /// Override from a dotted `key.path=value` string (the `--set` escape hatch). The
    /// value is parsed as a YAML scalar, so `true`/`123`/`1.5` get their natural types
    /// and anything else stays a string.
    pub fn set(dotted_key: &str, raw_value: &str) -> Overlay {
        let path = dotted_key.split('.').map(|s| s.to_string()).collect();
        let value = serde_norway::from_str::<serde_norway::Value>(raw_value)
            .unwrap_or_else(|_| serde_norway::Value::String(raw_value.to_string()));
        Overlay { path, value }
    }

    fn apply(&self, conf: &mut serde_norway::Value) {
        if !self.path.is_empty() {
            set_yaml_path(conf, &self.path, self.value.clone());
        }
    }
}

/// Set a (possibly nested) key path in a YAML mapping, creating intermediate maps.
fn set_yaml_path(node: &mut serde_norway::Value, path: &[String], value: serde_norway::Value) {
    use serde_norway::{Mapping, Value};
    if !node.is_mapping() {
        *node = Value::Mapping(Mapping::new());
    }
    let map = node.as_mapping_mut().expect("ensured mapping above");
    let key = Value::String(path[0].clone());
    if path.len() == 1 {
        map.insert(key, value);
        return;
    }
    let child = map
        .entry(key)
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    set_yaml_path(child, &path[1..], value);
}

/// Split `host:port` (also `[::1]:port` and raw `ip:port`) into parts; `None` if no
/// parseable port is present.
fn split_host_port(s: &str) -> Option<(String, u16)> {
    if let Ok(sa) = s.parse::<SocketAddr>() {
        return Some((sa.ip().to_string(), sa.port()));
    }
    let (host, port) = s.rsplit_once(':')?;
    let port: u16 = port.parse().ok()?;
    Some((host.to_string(), port))
}

/// `*` is a friendly spelling of "bind all interfaces"; `settings.yml` uses an empty ip.
fn normalize_host(host: &str) -> String {
    if host == "*" {
        String::new()
    } else {
        host.to_string()
    }
}

/// Lenient boolean parse (`true/false/1/0/yes/no/on/off`), à la itzg/minecraft-server.
fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "y" => Some(true),
        "false" | "0" | "no" | "off" | "n" => Some(false),
        _ => None,
    }
}

fn env_str(v: &mut Vec<Overlay>, key: &str, path: &[&str]) {
    if let Ok(val) = std::env::var(key) {
        v.push(Overlay::str(path, val));
    }
}
fn env_int(v: &mut Vec<Overlay>, key: &str, path: &[&str]) {
    if let Ok(val) = std::env::var(key)
        && let Ok(n) = val.trim().parse::<i64>()
    {
        v.push(Overlay::int(path, n));
    }
}
fn env_float(v: &mut Vec<Overlay>, key: &str, path: &[&str]) {
    if let Ok(val) = std::env::var(key)
        && let Ok(f) = val.trim().parse::<f64>()
    {
        v.push(Overlay::float(path, f));
    }
}
fn env_bool(v: &mut Vec<Overlay>, key: &str, path: &[&str]) {
    if let Ok(val) = std::env::var(key)
        && let Some(b) = parse_bool(&val)
    {
        v.push(Overlay::bool(path, b));
    }
}
/// A content var that also enables its section (e.g. `HOLLOW_BRAND` sets the brand text
/// and flips `brandName.enable` on).
fn env_section(v: &mut Vec<Overlay>, key: &str, content: &[&str], enable: &[&str]) {
    if let Ok(val) = std::env::var(key) {
        v.push(Overlay::str(content, val));
        v.push(Overlay::bool(enable, true));
    }
}

/// Collect overrides from `HOLLOW_*` environment variables — the whole `settings.yml`
/// surface. Applied in this order, so "content" vars auto-enable their section and the
/// explicit `*_ENABLE` toggles (pushed last) win.
pub fn env_overlays() -> Vec<Overlay> {
    let mut v: Vec<Overlay> = Vec::new();

    // bind / capacity
    if let Ok(bind) = std::env::var("HOLLOW_BIND")
        && let Some((h, p)) = split_host_port(&bind)
    {
        v.push(Overlay::str(&["bind", "ip"], normalize_host(&h)));
        v.push(Overlay::int(&["bind", "port"], p as i64));
    }
    if let Ok(h) = std::env::var("HOLLOW_HOST") {
        v.push(Overlay::str(&["bind", "ip"], normalize_host(&h)));
    }
    env_int(&mut v, "HOLLOW_PORT", &["bind", "port"]);
    env_int(&mut v, "HOLLOW_MAX_PLAYERS", &["maxPlayers"]);

    // ping / appearance
    if let Ok(m) = std::env::var("HOLLOW_MOTD").or_else(|_| std::env::var("HOLLOW_PING_DESCRIPTION")) {
        v.push(Overlay::str(&["ping", "description"], m));
    }
    env_str(&mut v, "HOLLOW_VERSION_NAME", &["ping", "version"]);
    env_int(&mut v, "HOLLOW_PROTOCOL", &["ping", "protocol"]);
    env_str(&mut v, "HOLLOW_DIMENSION", &["dimension"]);
    env_int(&mut v, "HOLLOW_GAMEMODE", &["gameMode"]);
    env_bool(&mut v, "HOLLOW_SECURE_PROFILE", &["secureProfile"]);

    // sections (content auto-enables)
    env_section(&mut v, "HOLLOW_BRAND", &["brandName", "content"], &["brandName", "enable"]);
    env_section(&mut v, "HOLLOW_JOIN_MESSAGE", &["joinMessage", "text"], &["joinMessage", "enable"]);
    env_section(&mut v, "HOLLOW_PLAYER_LIST_USERNAME", &["playerList", "username"], &["playerList", "enable"]);
    env_section(&mut v, "HOLLOW_HEADER", &["headerAndFooter", "header"], &["headerAndFooter", "enable"]);
    env_section(&mut v, "HOLLOW_FOOTER", &["headerAndFooter", "footer"], &["headerAndFooter", "enable"]);
    env_section(&mut v, "HOLLOW_BOSSBAR_TEXT", &["bossBar", "text"], &["bossBar", "enable"]);
    env_str(&mut v, "HOLLOW_BOSSBAR_COLOR", &["bossBar", "color"]);
    env_str(&mut v, "HOLLOW_BOSSBAR_DIVISION", &["bossBar", "division"]);
    env_float(&mut v, "HOLLOW_BOSSBAR_HEALTH", &["bossBar", "health"]);
    env_section(&mut v, "HOLLOW_TITLE", &["title", "title"], &["title", "enable"]);
    env_section(&mut v, "HOLLOW_SUBTITLE", &["title", "subtitle"], &["title", "enable"]);
    env_int(&mut v, "HOLLOW_TITLE_FADE_IN", &["title", "fadeIn"]);
    env_int(&mut v, "HOLLOW_TITLE_STAY", &["title", "stay"]);
    env_int(&mut v, "HOLLOW_TITLE_FADE_OUT", &["title", "fadeOut"]);

    // info forwarding
    if let Ok(t) = std::env::var("HOLLOW_FORWARDING").or_else(|_| std::env::var("HOLLOW_FORWARDING_TYPE")) {
        v.push(Overlay::str(&["infoForwarding", "type"], t));
    }
    env_str(&mut v, "HOLLOW_FORWARDING_SECRET", &["infoForwarding", "secret"]);
    if let Ok(t) = std::env::var("HOLLOW_FORWARDING_TOKENS") {
        let seq = t
            .split([',', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| serde_norway::Value::String(s.to_string()))
            .collect();
        v.push(Overlay::raw(&["infoForwarding", "tokens"], serde_norway::Value::Sequence(seq)));
    }

    // networking / logging
    env_int(&mut v, "HOLLOW_READ_TIMEOUT", &["readTimeout"]);
    env_int(&mut v, "HOLLOW_DEBUG_LEVEL", &["debugLevel"]);
    env_bool(&mut v, "HOLLOW_LOG_PLAYERS_IP", &["logPlayersIp"]);
    env_str(&mut v, "HOLLOW_TRANSPORT_TYPE", &["netty", "transportType"]);
    env_int(&mut v, "HOLLOW_WORKER_THREADS", &["netty", "threads", "workerGroup"]);

    // traffic limits
    env_bool(&mut v, "HOLLOW_TRAFFIC_ENABLE", &["traffic", "enable"]);
    env_int(&mut v, "HOLLOW_MAX_PACKET_SIZE", &["traffic", "maxPacketSize"]);
    env_float(&mut v, "HOLLOW_TRAFFIC_INTERVAL", &["traffic", "interval"]);
    env_float(&mut v, "HOLLOW_MAX_PACKET_RATE", &["traffic", "maxPacketRate"]);
    env_float(&mut v, "HOLLOW_MAX_PACKET_BYTES_RATE", &["traffic", "maxPacketBytesRate"]);

    // explicit enable toggles last, so they override the section-content auto-enable
    env_bool(&mut v, "HOLLOW_PLAYER_LIST", &["playerList", "enable"]);
    env_bool(&mut v, "HOLLOW_BRAND_ENABLE", &["brandName", "enable"]);
    env_bool(&mut v, "HOLLOW_JOIN_MESSAGE_ENABLE", &["joinMessage", "enable"]);
    env_bool(&mut v, "HOLLOW_HEADER_FOOTER_ENABLE", &["headerAndFooter", "enable"]);
    env_bool(&mut v, "HOLLOW_BOSSBAR_ENABLE", &["bossBar", "enable"]);
    env_bool(&mut v, "HOLLOW_TITLE_ENABLE", &["title", "enable"]);

    v
}

// ---------------------------------------------------------------------------
// Helper accessors on serde_norway::Value
// ---------------------------------------------------------------------------

/// Navigate a dotted/nested path.  Each segment is looked up as a YAML
/// mapping key.  Returns `None` if any segment is missing.
fn nav<'a>(v: &'a serde_norway::Value, keys: &[&str]) -> Option<&'a serde_norway::Value> {
    let mut cur = v;
    for &k in keys {
        cur = cur.as_mapping()?.get(k)?;
    }
    Some(cur)
}

// Scalar extractors ---------------------------------------------------------

fn get_str<'a>(v: &'a serde_norway::Value, keys: &[&str], default: &'a str) -> &'a str {
    nav(v, keys)
        .and_then(|n| n.as_str())
        .unwrap_or(default)
}

fn get_string(v: &serde_norway::Value, keys: &[&str], default: &str) -> String {
    get_str(v, keys, default).to_string()
}

fn get_bool(v: &serde_norway::Value, keys: &[&str], default: bool) -> bool {
    nav(v, keys)
        .and_then(|n| n.as_bool())
        .unwrap_or(default)
}

fn get_i32(v: &serde_norway::Value, keys: &[&str], default: i32) -> i32 {
    nav(v, keys)
        .and_then(|n| n.as_i64())
        .map(|x| x as i32)
        .unwrap_or(default)
}

fn get_i64(v: &serde_norway::Value, keys: &[&str], default: i64) -> i64 {
    nav(v, keys)
        .and_then(|n| n.as_i64())
        .unwrap_or(default)
}

fn get_f32(v: &serde_norway::Value, keys: &[&str], default: f32) -> f32 {
    nav(v, keys)
        .and_then(|n| n.as_f64())
        .map(|x| x as f32)
        .unwrap_or(default)
}

fn get_f64(v: &serde_norway::Value, keys: &[&str], default: f64) -> f64 {
    nav(v, keys)
        .and_then(|n| n.as_f64())
        .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Per-field parsing (one function per Java TypeSerializer)
// ---------------------------------------------------------------------------

// --- SocketAddressSerializer -----------------------------------------------
//
// bind:
//   ip: "localhost"
//   port: 65535
//
// Empty ip -> InetSocketAddress(port)  i.e. wildcard/unspecified address.
fn parse_address(conf: &serde_norway::Value) -> Result<SocketAddr, ConfigError> {
    let ip = get_string(conf, &["bind", "ip"], "");
    let port = get_i32(conf, &["bind", "port"], 0) as u16;

    resolve_addr(&ip, port)
}

/// Resolve an `(ip-or-hostname, port)` pair into a `SocketAddr`.
///
/// Empty ip binds all interfaces (Java's `InetSocketAddress(port)`); a raw IPv4/IPv6
/// literal is used directly; anything else is resolved via DNS, mirroring
/// `InetSocketAddress(host, port)` which resolves at construction time.
fn resolve_addr(ip: &str, port: u16) -> Result<SocketAddr, ConfigError> {
    if ip.is_empty() {
        return Ok(SocketAddr::new(IpAddr::from([0u8, 0, 0, 0]), port));
    }
    if let Ok(parsed) = ip.parse::<IpAddr>() {
        return Ok(SocketAddr::new(parsed, port));
    }
    use std::net::ToSocketAddrs;
    let addr_str = format!("{ip}:{port}");
    addr_str
        .to_socket_addrs()
        .map_err(|e| ConfigError::Semantic(format!("Cannot resolve bind address '{addr_str}': {e}")))?
        .next()
        .ok_or_else(|| ConfigError::Semantic(format!("No addresses resolved for '{addr_str}'")))
}

// --- ComponentSerializer ---------------------------------------------------
//
// node.getString("") passed to ComponentUtils.parse.
fn parse_component(v: &serde_norway::Value, keys: &[&str]) -> Component {
    let s = get_str(v, keys, "");
    hollow_text::parse(s)
}

// --- PingDataSerializer ----------------------------------------------------
fn parse_ping_data(conf: &serde_norway::Value) -> PingData {
    let description = parse_component(conf, &["ping", "description"]);
    let version = parse_component(conf, &["ping", "version"]);
    let protocol = get_i32(conf, &["ping", "protocol"], -1);
    PingData {
        description,
        version,
        protocol,
    }
}

// --- BossBarSerializer -----------------------------------------------------
//
// health must be 0.0..=1.0.
fn parse_boss_bar(
    conf: &serde_norway::Value,
) -> Result<BossBar, ConfigError> {
    let node = nav(conf, &["bossBar"])
        .ok_or_else(|| ConfigError::Semantic("Missing 'bossBar' section".into()))?;

    let text = {
        let s = get_str(node, &["text"], "");
        hollow_text::parse(s)
    };

    let health = get_f32(node, &["health"], 0.0);
    if !(0.0..=1.0).contains(&health) {
        return Err(ConfigError::Semantic(
            "BossBar health value must be between 0.0 and 1.0".into(),
        ));
    }

    let color_raw = get_string(node, &["color"], "").to_ascii_uppercase();
    let color = BossBarColor::from_name(&color_raw)
        .ok_or_else(|| ConfigError::Semantic(format!("Invalid bossbar color: '{color_raw}'")))?;

    let div_raw = get_string(node, &["division"], "").to_ascii_uppercase();
    let division = BossBarDivision::from_name(&div_raw)
        .ok_or_else(|| ConfigError::Semantic(format!("Invalid bossbar division: '{div_raw}'")))?;

    Ok(BossBar {
        text,
        health,
        color,
        division,
    })
}

// --- TitleSerializer -------------------------------------------------------
fn parse_title(conf: &serde_norway::Value) -> Title {
    let node_opt = nav(conf, &["title"]);
    let empty_val = serde_norway::Value::Null;
    let node = node_opt.unwrap_or(&empty_val);

    let title_comp = {
        let s = get_str(node, &["title"], "");
        hollow_text::parse(s)
    };
    let subtitle_comp = {
        let s = get_str(node, &["subtitle"], "");
        hollow_text::parse(s)
    };
    let fade_in = get_i32(node, &["fadeIn"], 10);
    let stay = get_i32(node, &["stay"], 100);
    let fade_out = get_i32(node, &["fadeOut"], 10);

    Title {
        title: title_comp,
        subtitle: subtitle_comp,
        fade_in,
        stay,
        fade_out,
    }
}

// --- InfoForwardingSerializer ----------------------------------------------
//
// @-prefix logic for both `secret` and `tokens` entries.
fn parse_info_forwarding(
    conf: &serde_norway::Value,
    root: &Path,
) -> Result<InfoForwarding, ConfigError> {
    let node = nav(conf, &["infoForwarding"])
        .ok_or_else(|| ConfigError::Semantic("Missing 'infoForwarding' section".into()))?;

    // Forwarding type
    let type_raw = get_string(node, &["type"], "").to_ascii_uppercase();
    let forwarding_type = match type_raw.as_str() {
        "NONE" => ForwardingType::None,
        "LEGACY" => ForwardingType::Legacy,
        "MODERN" => ForwardingType::Modern,
        "BUNGEE_GUARD" => ForwardingType::BungeeGuard,
        other => {
            return Err(ConfigError::Semantic(format!(
                "Undefined info forwarding type: '{other}'"
            )));
        }
    };

    // secret — only resolved when type == MODERN (mirrors Java logic)
    let secret_key: Vec<u8> = if forwarding_type == ForwardingType::Modern {
        let raw = get_string(node, &["secret"], "");
        resolve_secret(&raw, root)?
    } else {
        Vec::new()
    };

    // tokens — only resolved when type == BUNGEE_GUARD
    let tokens: Vec<String> = if forwarding_type == ForwardingType::BungeeGuard {
        resolve_tokens(node, root)?
    } else {
        Vec::new()
    };

    Ok(InfoForwarding {
        forwarding_type,
        secret_key,
        tokens,
    })
}

/// Port of `InfoForwardingSerializer.resolveSecret`.
///
/// If the value starts with `@` the file at `root/<rest>` is read and its
/// trimmed UTF-8 content is used as the secret bytes.
fn resolve_secret(raw: &str, root: &Path) -> Result<Vec<u8>, ConfigError> {
    if let Some(rel) = raw.strip_prefix('@') {
        let path = root.join(rel);
        let content = fs::read_to_string(&path).map_err(|e| {
            ConfigError::Semantic(format!(
                "Cannot read external secret file '{}': {e}",
                path.display()
            ))
        })?;
        Ok(content.trim().as_bytes().to_vec())
    } else {
        Ok(raw.as_bytes().to_vec())
    }
}

/// Port of `InfoForwardingSerializer.resolveTokens`.
///
/// The `tokens` YAML value can be:
///   - a list, where each entry is either an inline token or `@file.txt`
///   - a plain scalar `@file.txt` (whole file, one token per line)
///   - anything else -> empty list
fn resolve_tokens(
    node: &serde_norway::Value,
    root: &Path,
) -> Result<Vec<String>, ConfigError> {
    let tokens_val = match nav(node, &["tokens"]) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    if let Some(seq) = tokens_val.as_sequence() {
        // List form (may be empty)
        if seq.is_empty() {
            return Ok(Vec::new());
        }
        let mut resolved: Vec<String> = Vec::with_capacity(seq.len());
        for item in seq {
            let entry = item.as_str().unwrap_or("").to_string();
            if entry.starts_with('@') {
                resolved.extend(read_token_file(&entry, root)?);
            } else if !entry.is_empty() {
                resolved.push(entry);
            }
        }
        return Ok(resolved);
    }

    // Scalar form
    if let Some(scalar) = tokens_val.as_str()
        && scalar.starts_with('@')
    {
        return read_token_file(scalar, root);
    }

    Ok(Vec::new())
}

/// Port of `InfoForwardingSerializer.readLines`.
///
/// Read a file referenced by `@<path>`, returning non-blank, non-comment lines.
fn read_token_file(entry: &str, root: &Path) -> Result<Vec<String>, ConfigError> {
    let rel = entry.trim_start_matches('@');
    let path = root.join(rel);
    let content = fs::read_to_string(&path).map_err(|e| {
        ConfigError::Semantic(format!(
            "Cannot read external tokens file '{}': {e}",
            path.display()
        ))
    })?;
    let lines = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    Ok(lines)
}

// ---------------------------------------------------------------------------
// Top-level parse
// ---------------------------------------------------------------------------

fn parse_config(
    conf: &serde_norway::Value,
    root: &Path,
) -> Result<LimboConfig, ConfigError> {
    // bind -> SocketAddr
    let address = parse_address(conf)?;

    // maxPlayers (default 100)
    let max_players = get_i32(conf, &["maxPlayers"], 100);

    // ping
    let ping_data = parse_ping_data(conf);

    // dimension (default THE_END) — tolerate missing/empty by defaulting
    let dimension_type = {
        let raw = get_string(conf, &["dimension"], "").to_ascii_uppercase();
        if raw.is_empty() {
            DimensionType::TheEnd
        } else {
            let canonical = match raw.as_str() {
                "NETHER" => "THE_NETHER",
                "END" => "THE_END",
                other => other,
            };
            DimensionType::from_name(canonical).unwrap_or(DimensionType::TheEnd)
        }
    };

    // gameMode (default 3)
    let game_mode = get_i32(conf, &["gameMode"], 3);

    // secureProfile (default false)
    let secure_profile = get_bool(conf, &["secureProfile"], false);

    // brandName.enable / brandName.content
    let use_brand_name = get_bool(conf, &["brandName", "enable"], false);
    let brand_name = if use_brand_name {
        parse_component(conf, &["brandName", "content"])
    } else {
        Component::empty()
    };

    // joinMessage.enable / joinMessage.text
    let use_join_message = get_bool(conf, &["joinMessage", "enable"], false);
    let join_message = if use_join_message {
        parse_component(conf, &["joinMessage", "text"])
    } else {
        Component::empty()
    };

    // bossBar.enable
    let use_boss_bar = get_bool(conf, &["bossBar", "enable"], false);
    let boss_bar = if use_boss_bar {
        Some(parse_boss_bar(conf)?)
    } else {
        None
    };

    // title.enable
    let use_title = get_bool(conf, &["title", "enable"], false);
    let title = if use_title {
        Some(parse_title(conf))
    } else {
        None
    };

    // playerList.enable / playerList.username
    let use_player_list = get_bool(conf, &["playerList", "enable"], false);
    let player_list_username = get_string(conf, &["playerList", "username"], "");

    // headerAndFooter.enable / header / footer
    let use_header_and_footer = get_bool(conf, &["headerAndFooter", "enable"], false);
    let (player_list_header, player_list_footer) = if use_header_and_footer {
        (
            parse_component(conf, &["headerAndFooter", "header"]),
            parse_component(conf, &["headerAndFooter", "footer"]),
        )
    } else {
        (Component::empty(), Component::empty())
    };

    // infoForwarding
    let info_forwarding = parse_info_forwarding(conf, root)?;

    // readTimeout (default 30000)
    let read_timeout = get_i64(conf, &["readTimeout"], 30000);

    // debugLevel (default 2)
    let debug_level = get_i32(conf, &["debugLevel"], 2);

    // logPlayersIp (default true)
    let log_players_ip = get_bool(conf, &["logPlayersIp"], true);

    // netty.transportType (default EPOLL)
    let transport_type = {
        let raw = get_string(conf, &["netty", "transportType"], "").to_ascii_uppercase();
        if raw.is_empty() {
            TransportType::Epoll
        } else {
            TransportType::from_name(&raw).unwrap_or(TransportType::Epoll)
        }
    };

    // netty.threads.bossGroup (default 1) / workerGroup (default 4)
    let boss_group_size = get_i32(conf, &["netty", "threads", "bossGroup"], 1);
    let worker_group_size = get_i32(conf, &["netty", "threads", "workerGroup"], 4);

    // traffic.enable (default false)
    let use_traffic_limits = get_bool(conf, &["traffic", "enable"], false);
    let max_packet_size = get_i32(conf, &["traffic", "maxPacketSize"], -1);
    let interval = get_f64(conf, &["traffic", "interval"], -1.0);
    let max_packet_rate = get_f64(conf, &["traffic", "maxPacketRate"], -1.0);
    let max_packet_bytes_rate = get_f64(conf, &["traffic", "maxPacketBytesRate"], -1.0);

    Ok(LimboConfig {
        address,
        max_players,
        ping_data,
        dimension_type,
        game_mode,
        secure_profile,
        use_brand_name,
        use_join_message,
        use_boss_bar,
        use_title,
        use_player_list,
        use_header_and_footer,
        brand_name,
        join_message,
        boss_bar,
        title,
        player_list_username,
        player_list_header,
        player_list_footer,
        info_forwarding,
        read_timeout,
        debug_level,
        log_players_ip,
        transport_type,
        boss_group_size,
        worker_group_size,
        use_traffic_limits,
        max_packet_size,
        interval,
        max_packet_rate,
        max_packet_bytes_rate,
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_norway;

    fn default_conf() -> serde_norway::Value {
        serde_norway::from_str(DEFAULT_SETTINGS).expect("default settings must parse")
    }

    #[test]
    fn default_max_players() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["maxPlayers"], 100), 100);
    }

    #[test]
    fn default_game_mode() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["gameMode"], 3), 3);
    }

    #[test]
    fn default_dimension() {
        let c = default_conf();
        let raw = get_string(&c, &["dimension"], "").to_ascii_uppercase();
        let canonical = match raw.as_str() {
            "NETHER" => "THE_NETHER",
            "END" => "THE_END",
            other => other,
        };
        assert_eq!(
            DimensionType::from_name(canonical),
            Some(DimensionType::TheEnd)
        );
    }

    #[test]
    fn default_read_timeout() {
        let c = default_conf();
        assert_eq!(get_i64(&c, &["readTimeout"], 30000), 30000);
    }

    #[test]
    fn default_debug_level() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["debugLevel"], 2), 2);
    }

    #[test]
    fn default_log_players_ip() {
        let c = default_conf();
        assert!(get_bool(&c, &["logPlayersIp"], true));
    }

    #[test]
    fn default_boss_group() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["netty", "threads", "bossGroup"], 1), 1);
    }

    #[test]
    fn default_worker_group() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["netty", "threads", "workerGroup"], 4), 4);
    }

    #[test]
    fn default_protocol() {
        let c = default_conf();
        assert_eq!(get_i32(&c, &["ping", "protocol"], -1), -1);
    }

    #[test]
    fn default_boss_bar() {
        let c = default_conf();
        assert!(get_bool(&c, &["bossBar", "enable"], false));
        let bb = parse_boss_bar(&c).expect("bossBar must parse");
        assert!((bb.health - 1.0f32).abs() < 1e-6);
        assert_eq!(bb.color, BossBarColor::Blue);
        assert_eq!(bb.division, BossBarDivision::Solid);
    }

    #[test]
    fn default_title() {
        let c = default_conf();
        assert!(get_bool(&c, &["title", "enable"], false));
        let t = parse_title(&c);
        assert_eq!(t.fade_in, 10);
        assert_eq!(t.stay, 100);
        assert_eq!(t.fade_out, 10);
    }

    #[test]
    fn default_traffic() {
        let c = default_conf();
        // The default settings.yml has traffic.enable: true
        assert!(get_bool(&c, &["traffic", "enable"], false));
        assert_eq!(get_i32(&c, &["traffic", "maxPacketSize"], -1), 8192);
        assert!((get_f64(&c, &["traffic", "interval"], -1.0) - 7.0).abs() < 1e-9);
        assert!((get_f64(&c, &["traffic", "maxPacketRate"], -1.0) - 500.0).abs() < 1e-9);
        assert!((get_f64(&c, &["traffic", "maxPacketBytesRate"], -1.0) - 2048.0).abs() < 1e-9);
    }

    #[test]
    fn address_empty_ip_is_wildcard() {
        let yaml = "bind:\n  ip: \"\"\n  port: 25565\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let addr = parse_address(&c).unwrap();
        assert_eq!(addr.port(), 25565);
        assert!(addr.ip().is_unspecified());
    }

    #[test]
    fn address_localhost() {
        let yaml = "bind:\n  ip: \"127.0.0.1\"\n  port: 25565\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let addr = parse_address(&c).unwrap();
        assert_eq!(addr.port(), 25565);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn forwarding_none() {
        let yaml = "infoForwarding:\n  type: NONE\n  secret: \"\"\n  tokens:\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let fwd = parse_info_forwarding(&c, Path::new(".")).unwrap();
        assert_eq!(fwd.forwarding_type, ForwardingType::None);
        assert!(fwd.secret_key.is_empty());
        assert!(fwd.tokens.is_empty());
    }

    #[test]
    fn forwarding_modern_inline_secret() {
        let yaml =
            "infoForwarding:\n  type: MODERN\n  secret: \"mysecret\"\n  tokens:\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let fwd = parse_info_forwarding(&c, Path::new(".")).unwrap();
        assert_eq!(fwd.forwarding_type, ForwardingType::Modern);
        assert_eq!(fwd.secret_key, b"mysecret");
    }

    #[test]
    fn forwarding_bungee_guard_inline_tokens() {
        let yaml =
            "infoForwarding:\n  type: BUNGEE_GUARD\n  secret: \"\"\n  tokens:\n    - tok1\n    - tok2\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let fwd = parse_info_forwarding(&c, Path::new(".")).unwrap();
        assert_eq!(fwd.forwarding_type, ForwardingType::BungeeGuard);
        assert_eq!(fwd.tokens, vec!["tok1".to_string(), "tok2".to_string()]);
    }

    #[test]
    fn dimension_alias_nether() {
        let yaml = "dimension: NETHER\n";
        let c: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        let raw = get_string(&c, &["dimension"], "").to_ascii_uppercase();
        let canonical = match raw.as_str() {
            "NETHER" => "THE_NETHER",
            "END" => "THE_END",
            other => other,
        };
        assert_eq!(
            DimensionType::from_name(canonical),
            Some(DimensionType::TheNether)
        );
    }
}
