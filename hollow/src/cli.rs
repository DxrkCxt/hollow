// SPDX-License-Identifier: GPL-3.0-only

//! Tiny hand-rolled CLI parser (no external dependency, in the spirit of the rest of
//! the project). Produces an [`Action`] for `main` to act on.
//!
//! Flags build [`Overlay`]s that are layered on top of `settings.yml`. Every flag also
//! has a `HOLLOW_*` environment counterpart (see
//! [`hollow::configuration::env_overlays`]); precedence is CLI > env > file > defaults.
//! The `--set <key.path>=<value>` escape hatch can override *any* settings.yml key.

use std::path::PathBuf;

use hollow::configuration::Overlay;

pub const HELP: &str = "\
Hollow — a lightweight Minecraft limbo server

USAGE:
    hollow [OPTIONS]

COMMON OPTIONS:
    -c, --config <FILE>          Path to settings.yml (default: ./settings.yml; created if missing)
        --bind <HOST:PORT>       Bind address, e.g. 0.0.0.0:25565 or [::]:25565
        --host <HOST>            Bind host/IP only ('*' = all interfaces)
    -p, --port <PORT>            Bind port only
    -m, --max-players <N>        Max players (0/-1 = unlimited)
        --motd <TEXT>            Server list description (MiniMessage)
        --dimension <DIM>        OVERWORLD | THE_NETHER | THE_END
        --gamemode <0-3>         0 survival, 1 creative, 2 adventure, 3 spectator
        --forwarding <TYPE>      NONE | LEGACY | MODERN | BUNGEE_GUARD
        --forwarding-secret <S>  Velocity modern-forwarding secret ('@file' supported)
        --debug-level <0-3>      Log verbosity
        --set <KEY=VALUE>        Override any settings.yml key (dotted), repeatable
                                 e.g. --set traffic.maxPacketRate=800
    -h, --help                   Print help and exit
    -V, --version                Print version and exit
        --healthcheck            Probe the configured port and exit 0 (up) / 1 (down)

ENVIRONMENT (CLI takes precedence):
    HOLLOW_CONFIG  HOLLOW_BIND  HOLLOW_HOST  HOLLOW_PORT  HOLLOW_MAX_PLAYERS
    HOLLOW_MOTD  HOLLOW_VERSION_NAME  HOLLOW_PROTOCOL  HOLLOW_DIMENSION  HOLLOW_GAMEMODE
    HOLLOW_SECURE_PROFILE  HOLLOW_BRAND  HOLLOW_JOIN_MESSAGE  HOLLOW_PLAYER_LIST_USERNAME
    HOLLOW_HEADER  HOLLOW_FOOTER  HOLLOW_BOSSBAR_TEXT/COLOR/DIVISION/HEALTH
    HOLLOW_TITLE  HOLLOW_SUBTITLE  HOLLOW_TITLE_FADE_IN/STAY/FADE_OUT
    HOLLOW_FORWARDING[_TYPE]  HOLLOW_FORWARDING_SECRET  HOLLOW_FORWARDING_TOKENS
    HOLLOW_READ_TIMEOUT  HOLLOW_DEBUG_LEVEL  HOLLOW_LOG_PLAYERS_IP
    HOLLOW_TRANSPORT_TYPE  HOLLOW_WORKER_THREADS
    HOLLOW_TRAFFIC_ENABLE  HOLLOW_MAX_PACKET_SIZE  HOLLOW_TRAFFIC_INTERVAL
    HOLLOW_MAX_PACKET_RATE  HOLLOW_MAX_PACKET_BYTES_RATE
    *_ENABLE toggles: HOLLOW_BRAND_ENABLE, HOLLOW_BOSSBAR_ENABLE, HOLLOW_TITLE_ENABLE,
    HOLLOW_JOIN_MESSAGE_ENABLE, HOLLOW_HEADER_FOOTER_ENABLE, HOLLOW_PLAYER_LIST

Precedence (highest first): CLI flags > environment > settings.yml > built-in defaults.
Spin up many instances from one image, e.g.:
    HOLLOW_PORT=25566 hollow        # or: hollow --port 25566
";

/// What `main` should do after parsing arguments.
pub enum Action {
    Run {
        config_path: PathBuf,
        overlays: Vec<Overlay>,
    },
    /// TCP-probe the configured port and exit 0 (reachable) / 1 (not).
    Healthcheck {
        config_path: PathBuf,
        overlays: Vec<Overlay>,
    },
    PrintHelp,
    PrintVersion,
}

/// A user-facing argument error.
pub struct CliError(pub String);

fn take_value(
    argv: &[String],
    i: &mut usize,
    inline: &Option<String>,
    key: &str,
) -> Result<String, CliError> {
    if let Some(v) = inline {
        return Ok(v.clone());
    }
    *i += 1;
    argv.get(*i)
        .cloned()
        .ok_or_else(|| CliError(format!("missing value for {key}")))
}

fn parse_num<T: std::str::FromStr>(label: &str, v: &str) -> Result<T, CliError> {
    v.trim()
        .parse::<T>()
        .map_err(|_| CliError(format!("invalid {label}: '{v}'")))
}

/// Parse process arguments (including `argv[0]`) into an [`Action`].
pub fn parse(argv: Vec<String>) -> Result<Action, CliError> {
    let mut config: Option<PathBuf> = None;
    let mut overlays: Vec<Overlay> = Vec::new();
    let mut healthcheck = false;

    let mut i = 1; // skip argv[0]
    while i < argv.len() {
        let raw = &argv[i];
        // Support the `--key=value` form.
        let (key, inline) = match raw.split_once('=') {
            Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
            _ => (raw.clone(), None),
        };

        match key.as_str() {
            "-h" | "--help" => return Ok(Action::PrintHelp),
            "-V" | "--version" => return Ok(Action::PrintVersion),
            "--healthcheck" => healthcheck = true,
            "-c" | "--config" => {
                config = Some(PathBuf::from(take_value(&argv, &mut i, &inline, &key)?));
            }
            "--bind" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                let (host, port) = if let Ok(sa) = v.parse::<std::net::SocketAddr>() {
                    (sa.ip().to_string(), sa.port())
                } else if let Some((h, p)) = v.rsplit_once(':') {
                    (h.to_string(), parse_num("port in --bind", p)?)
                } else {
                    return Err(CliError(format!("--bind expects HOST:PORT, got '{v}'")));
                };
                let host = if host == "*" { String::new() } else { host };
                overlays.push(Overlay::str(&["bind", "ip"], host));
                overlays.push(Overlay::int(&["bind", "port"], port as i64));
            }
            "--host" => {
                let h = take_value(&argv, &mut i, &inline, &key)?;
                let h = if h == "*" { String::new() } else { h };
                overlays.push(Overlay::str(&["bind", "ip"], h));
            }
            "-p" | "--port" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::int(&["bind", "port"], parse_num::<u16>("port", &v)? as i64));
            }
            "-m" | "--max-players" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::int(&["maxPlayers"], parse_num::<i64>("max-players", &v)?));
            }
            "--motd" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::str(&["ping", "description"], v));
            }
            "--dimension" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::str(&["dimension"], v));
            }
            "--gamemode" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::int(&["gameMode"], parse_num::<i64>("gamemode", &v)?));
            }
            "--forwarding" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::str(&["infoForwarding", "type"], v));
            }
            "--forwarding-secret" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::str(&["infoForwarding", "secret"], v));
            }
            "--debug-level" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                overlays.push(Overlay::int(&["debugLevel"], parse_num::<i64>("debug-level", &v)?));
            }
            "--set" => {
                let v = take_value(&argv, &mut i, &inline, &key)?;
                let (k, val) = v
                    .split_once('=')
                    .ok_or_else(|| CliError(format!("--set expects KEY=VALUE, got '{v}'")))?;
                overlays.push(Overlay::set(k, val));
            }
            other => return Err(CliError(format!("unknown argument '{other}'"))),
        }
        i += 1;
    }

    let config_path = config
        .or_else(|| std::env::var_os("HOLLOW_CONFIG").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("./settings.yml"));

    if healthcheck {
        Ok(Action::Healthcheck {
            config_path,
            overlays,
        })
    } else {
        Ok(Action::Run {
            config_path,
            overlays,
        })
    }
}
