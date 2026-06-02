// SPDX-License-Identifier: GPL-3.0-only

//! Load benchmark — emulates many players joining the limbo server.
//!
//! Each emulated player performs the full join sequence
//! (handshake -> login -> [configuration] -> play), exactly like a real client,
//! using packet ids resolved from the server's own registry (so it tracks any
//! supported protocol, not just hard-coded 1.21). It measures join throughput and
//! latency, and can hold the connections in Play (answering keep-alives) to emulate
//! a sustained player population.
//!
//! Requirements on the target server:
//!   * `infoForwarding.type: NONE` (Velocity/BungeeGuard handshakes can't be forged here)
//!   * default flow targets modern protocols (>= 1.20.2; default 767 = 1.21)
//!
//! Run it (release is important for meaningful numbers):
//!   cargo run --release --example bench -- --players 5000 --concurrency 500
//!   cargo run --release --example bench -- --addr 127.0.0.1:25565 --players 2000 --hold 30
//!
//! Flags:
//!   --addr <host:port>    server address                 (default [::1]:65535)
//!   --players <N>         total players to emulate       (default 1000)
//!   --concurrency <C>     max simultaneous joins (ramp)  (default 256)
//!   --protocol <P>        MC protocol number             (default 767 = 1.21)
//!   --hold <SECS>         keep players in Play this long, echoing keep-alives
//!                         (default 0 = disconnect right after joining)
//!   --name-prefix <S>     username prefix (must stay unique per player; default Bench)

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Semaphore;

use hollow::protocol::registry::{PacketKind, State, Version, registry};

// ---------------------------------------------------------------------------
// Protocol helpers
// ---------------------------------------------------------------------------

fn write_varint(buf: &mut Vec<u8>, mut v: i32) {
    loop {
        let mut b = (v & 0x7F) as u8;
        v = ((v as u32) >> 7) as i32;
        if v != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if v == 0 {
            break;
        }
    }
}

fn frame(id: i32, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(body.len() + 2);
    write_varint(&mut payload, id);
    payload.extend_from_slice(body);
    let mut out = Vec::with_capacity(payload.len() + 3);
    write_varint(&mut out, payload.len() as i32);
    out.extend_from_slice(&payload);
    out
}

fn read_varint_slice(buf: &[u8]) -> Option<(i32, usize)> {
    let mut value = 0i32;
    let mut pos = 0;
    let mut i = 0;
    loop {
        let b = *buf.get(i)?;
        i += 1;
        value |= ((b & 0x7F) as i32) << pos;
        if b & 0x80 == 0 {
            return Some((value, i));
        }
        pos += 7;
        if pos >= 35 {
            return None;
        }
    }
}

async fn read_varint<R: AsyncReadExt + Unpin>(r: &mut R, wire: &mut u64) -> io::Result<i32> {
    let mut value = 0i32;
    let mut pos = 0;
    loop {
        let b = r.read_u8().await?;
        *wire += 1;
        value |= ((b & 0x7F) as i32) << pos;
        if b & 0x80 == 0 {
            return Ok(value);
        }
        pos += 7;
        if pos >= 35 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "varint too long"));
        }
    }
}

/// Read one length-prefixed packet. Returns (id, body, bytes-read-off-the-wire).
async fn read_packet<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<(i32, Vec<u8>, u64)> {
    let mut wire = 0u64;
    let len = read_varint(r, &mut wire).await? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    wire += len as u64;
    let (id, consumed) = read_varint_slice(&buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad packet id varint"))?;
    Ok((id, buf[consumed..].to_vec(), wire))
}

// ---------------------------------------------------------------------------
// Resolved packet ids (from the crate's own registry)
// ---------------------------------------------------------------------------

struct Ids {
    ver: Version,
    modern: bool,
    // serverbound (we send)
    sb_handshake: i32,
    sb_login_start: i32,
    sb_login_ack: Option<i32>,
    sb_known_packs: Option<i32>,
    sb_finish_config: Option<i32>,
    sb_keep_alive: i32,
    // clientbound (we detect)
    cb_login_success: i32,
    cb_known_packs: Option<i32>,
    cb_finish_config: Option<i32>,
    cb_join_game: i32,
    cb_keep_alive: i32,
}

fn resolve_ids(protocol: i32) -> Result<Ids, String> {
    let ver = Version::of(protocol);
    if !ver.is_supported() {
        return Err(format!("protocol {protocol} maps to unsupported version {ver:?}"));
    }
    let r = registry();
    let sb = |st, k| r.server_registry(st, ver).and_then(|reg| reg.get_id(k));
    let cb = |st, k| r.client_registry(st, ver).and_then(|reg| reg.get_id(k));

    Ok(Ids {
        ver,
        modern: ver.more_or_equal(Version::V1_20_2),
        sb_handshake: sb(State::Handshaking, PacketKind::Handshake)
            .ok_or("no serverbound handshake id")?,
        sb_login_start: sb(State::Login, PacketKind::LoginStart)
            .ok_or("no serverbound login-start id")?,
        sb_login_ack: sb(State::Login, PacketKind::LoginAcknowledged),
        sb_known_packs: sb(State::Configuration, PacketKind::KnownPacks),
        sb_finish_config: sb(State::Configuration, PacketKind::FinishConfiguration),
        sb_keep_alive: sb(State::Play, PacketKind::KeepAlive)
            .ok_or("no serverbound play keep-alive id")?,
        cb_login_success: cb(State::Login, PacketKind::LoginSuccess)
            .ok_or("no clientbound login-success id")?,
        cb_known_packs: cb(State::Configuration, PacketKind::KnownPacks),
        cb_finish_config: cb(State::Configuration, PacketKind::FinishConfiguration),
        cb_join_game: cb(State::Play, PacketKind::JoinGame)
            .ok_or("no clientbound play join-game id")?,
        cb_keep_alive: cb(State::Play, PacketKind::KeepAlive)
            .ok_or("no clientbound play keep-alive id")?,
    })
}

// ---------------------------------------------------------------------------
// Config / shared stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Cfg {
    addr: String,
    host: String,
    port: u16,
    players: usize,
    concurrency: usize,
    protocol: i32,
    hold: Duration,
    name_prefix: String,
}

#[derive(Default)]
struct Stats {
    live: AtomicUsize,
    joined_total: AtomicUsize,
    errors: AtomicUsize,
    peak: AtomicUsize,
}

#[derive(Default, Clone)]
struct PlayerOutcome {
    connect_latency: Option<Duration>,
    login_latency: Option<Duration>,
    join_latency: Option<Duration>,
    bytes: u64,
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// One emulated player
// ---------------------------------------------------------------------------

type Reader = BufReader<OwnedReadHalf>;

async fn do_join(
    start: &Instant,
    o: &mut PlayerOutcome,
    cfg: &Cfg,
    ids: &Ids,
    name: &str,
) -> Result<(Reader, OwnedWriteHalf), String> {
    let stream = TcpStream::connect(&cfg.addr)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    o.connect_latency = Some(start.elapsed());
    let _ = stream.set_nodelay(true);
    let (rd, mut wr) = stream.into_split();
    let mut rd = BufReader::new(rd);

    // Handshake (intent = login = 2).
    let mut hb = Vec::new();
    write_varint(&mut hb, ids.ver.protocol_number());
    write_varint(&mut hb, cfg.host.len() as i32);
    hb.extend_from_slice(cfg.host.as_bytes());
    hb.extend_from_slice(&cfg.port.to_be_bytes());
    write_varint(&mut hb, 2);
    wr.write_all(&frame(ids.sb_handshake, &hb))
        .await
        .map_err(|e| format!("handshake: {e}"))?;

    // Login start: username + uuid (server derives the real offline uuid from the name).
    let mut lb = Vec::new();
    write_varint(&mut lb, name.len() as i32);
    lb.extend_from_slice(name.as_bytes());
    lb.extend_from_slice(&[0u8; 16]);
    wr.write_all(&frame(ids.sb_login_start, &lb))
        .await
        .map_err(|e| format!("login-start: {e}"))?;

    // Read until Login Success.
    loop {
        let (id, _body, w) = read_packet(&mut rd)
            .await
            .map_err(|e| format!("login-read: {e}"))?;
        o.bytes += w;
        if id == ids.cb_login_success {
            break;
        }
    }
    o.login_latency = Some(start.elapsed());

    // Configuration phase (1.20.2+): ack, answer known-packs, await finish-config.
    if ids.modern {
        let ack = ids.sb_login_ack.ok_or("missing login-ack id")?;
        wr.write_all(&frame(ack, &[]))
            .await
            .map_err(|e| format!("login-ack: {e}"))?;
        loop {
            let (id, _b, w) = read_packet(&mut rd)
                .await
                .map_err(|e| format!("config-read: {e}"))?;
            o.bytes += w;
            if Some(id) == ids.cb_known_packs {
                let kp = ids.sb_known_packs.ok_or("missing known-packs id")?;
                let mut body = Vec::new();
                write_varint(&mut body, 0); // 0 known packs
                wr.write_all(&frame(kp, &body))
                    .await
                    .map_err(|e| format!("known-packs: {e}"))?;
            } else if Some(id) == ids.cb_finish_config {
                let fc = ids.sb_finish_config.ok_or("missing finish-config id")?;
                wr.write_all(&frame(fc, &[]))
                    .await
                    .map_err(|e| format!("finish-config: {e}"))?;
                break;
            }
        }
    }

    // Play phase: read until Join Game — at which point the player has fully joined.
    loop {
        let (id, _b, w) = read_packet(&mut rd)
            .await
            .map_err(|e| format!("play-read: {e}"))?;
        o.bytes += w;
        if id == ids.cb_join_game {
            break;
        }
    }
    o.join_latency = Some(start.elapsed());

    Ok((rd, wr))
}

async fn hold_loop(
    rd: &mut Reader,
    wr: &mut OwnedWriteHalf,
    o: &mut PlayerOutcome,
    ids: &Ids,
    hold: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + hold;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        match tokio::time::timeout(remaining, read_packet(rd)).await {
            Ok(Ok((id, body, w))) => {
                o.bytes += w;
                if id == ids.cb_keep_alive {
                    // Echo the 8-byte keep-alive payload back so the server's read
                    // timeout never fires.
                    wr.write_all(&frame(ids.sb_keep_alive, &body))
                        .await
                        .map_err(|e| format!("ka-echo: {e}"))?;
                }
            }
            Ok(Err(e)) => return Err(format!("hold-read: {e}")),
            Err(_) => return Ok(()), // timeout reached -> hold window elapsed
        }
    }
}

async fn run_player(
    permit: tokio::sync::OwnedSemaphorePermit,
    name: String,
    cfg: Arc<Cfg>,
    ids: Arc<Ids>,
    stats: Arc<Stats>,
) -> PlayerOutcome {
    let start = Instant::now();
    let mut o = PlayerOutcome::default();

    let join = do_join(&start, &mut o, &cfg, &ids, &name).await;
    // Free the ramp slot as soon as the join resolves (success or failure), so
    // concurrency caps simultaneous *joins*, not the held population.
    drop(permit);

    let (mut rd, mut wr) = match join {
        Ok(halves) => halves,
        Err(e) => {
            o.error = Some(e);
            stats.errors.fetch_add(1, Ordering::Relaxed);
            return o;
        }
    };

    stats.joined_total.fetch_add(1, Ordering::Relaxed);
    stats.live.fetch_add(1, Ordering::Relaxed);

    if cfg.hold > Duration::ZERO
        && let Err(e) = hold_loop(&mut rd, &mut wr, &mut o, &ids, cfg.hold).await
    {
        o.error = Some(e);
        stats.errors.fetch_add(1, Ordering::Relaxed);
    }

    stats.live.fetch_sub(1, Ordering::Relaxed);
    o
}

// ---------------------------------------------------------------------------
// Argument parsing + main
// ---------------------------------------------------------------------------

fn next_val(args: &[String], i: &mut usize) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("missing value for {}", args[*i - 1]))
}

fn parse_args() -> Result<Cfg, String> {
    let mut addr = "[::1]:65535".to_string();
    let mut players = 1000usize;
    let mut concurrency = 256usize;
    let mut protocol = 767i32;
    let mut hold = 0u64;
    let mut name_prefix = "Bench".to_string();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" => addr = next_val(&args, &mut i)?,
            "--players" => {
                players = next_val(&args, &mut i)?.parse().map_err(|_| "bad --players")?
            }
            "--concurrency" => {
                concurrency = next_val(&args, &mut i)?.parse().map_err(|_| "bad --concurrency")?
            }
            "--protocol" => {
                protocol = next_val(&args, &mut i)?.parse().map_err(|_| "bad --protocol")?
            }
            "--hold" => hold = next_val(&args, &mut i)?.parse().map_err(|_| "bad --hold")?,
            "--name-prefix" => name_prefix = next_val(&args, &mut i)?,
            "--help" | "-h" => return Err("help".to_string()),
            other => return Err(format!("unknown flag: {other}")),
        }
        i += 1;
    }

    if players == 0 {
        return Err("--players must be > 0".to_string());
    }
    let concurrency = concurrency.clamp(1, players);
    let port = addr
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(25565u16);

    Ok(Cfg {
        addr,
        host: "localhost".to_string(),
        port,
        players,
        concurrency,
        protocol,
        hold: Duration::from_secs(hold),
        name_prefix,
    })
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[idx]
}

#[tokio::main]
async fn main() {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            if e != "help" {
                eprintln!("error: {e}\n");
            }
            eprintln!("{}", env!("CARGO_PKG_NAME"));
            eprintln!(
                "usage: cargo run --release --example bench -- \
                 [--addr H:P] [--players N] [--concurrency C] \
                 [--protocol P] [--hold SECS] [--name-prefix S]"
            );
            std::process::exit(if e == "help" { 0 } else { 2 });
        }
    };

    let ids = match resolve_ids(cfg.protocol) {
        Ok(i) => Arc::new(i),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    if cfg!(debug_assertions) {
        eprintln!("WARNING: built without --release; numbers will be misleading.\n");
    }

    println!("Hollow load benchmark");
    println!("  target       {}", cfg.addr);
    println!(
        "  protocol     {} ({:?}{})",
        cfg.protocol,
        ids.ver,
        if ids.modern { ", configuration phase" } else { "" }
    );
    println!("  players      {}", cfg.players);
    println!("  concurrency  {}", cfg.concurrency);
    println!(
        "  hold         {}",
        if cfg.hold.is_zero() {
            "disconnect after join".to_string()
        } else {
            format!("{}s in Play (keep-alive echo)", cfg.hold.as_secs())
        }
    );
    println!();

    let cfg = Arc::new(cfg);
    let stats = Arc::new(Stats::default());
    let sem = Arc::new(Semaphore::new(cfg.concurrency));
    let bench_start = Instant::now();

    // Live progress line.
    let monitor = {
        let stats = stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;
                let live = stats.live.load(Ordering::Relaxed);
                if live > stats.peak.load(Ordering::Relaxed) {
                    stats.peak.store(live, Ordering::Relaxed);
                }
                eprintln!(
                    "[{:>6.1}s] joined={} live={} peak={} errors={}",
                    bench_start.elapsed().as_secs_f64(),
                    stats.joined_total.load(Ordering::Relaxed),
                    live,
                    stats.peak.load(Ordering::Relaxed),
                    stats.errors.load(Ordering::Relaxed),
                );
            }
        })
    };

    // Ramp: spawn players, gated by the concurrency semaphore.
    let mut handles = Vec::with_capacity(cfg.players);
    for i in 0..cfg.players {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let name = format!("{}-{i}", cfg.name_prefix);
        handles.push(tokio::spawn(run_player(
            permit,
            name,
            cfg.clone(),
            ids.clone(),
            stats.clone(),
        )));
    }

    let mut outcomes = Vec::with_capacity(handles.len());
    for h in handles {
        outcomes.push(h.await.unwrap_or_default());
    }
    monitor.abort();
    let wall = bench_start.elapsed();

    // ---- aggregate ----
    let n_joined = outcomes.iter().filter(|o| o.join_latency.is_some()).count();
    let n_failed = outcomes.len() - n_joined;
    let total_bytes: u64 = outcomes.iter().map(|o| o.bytes).sum();

    let collect_ms = |f: &dyn Fn(&PlayerOutcome) -> Option<Duration>| -> Vec<f64> {
        let mut v: Vec<f64> = outcomes
            .iter()
            .filter_map(f)
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    };
    let connect_ms = collect_ms(&|o| o.connect_latency);
    let login_ms = collect_ms(&|o| o.login_latency);
    let join_ms = collect_ms(&|o| o.join_latency);

    let mut hist: HashMap<&str, usize> = HashMap::new();
    for o in &outcomes {
        if let Some(e) = &o.error {
            *hist.entry(e.split(':').next().unwrap_or("error")).or_default() += 1;
        }
    }

    let secs = wall.as_secs_f64().max(1e-9);
    println!("\n==================== results ====================");
    println!("wall time          {:.2}s", secs);
    println!(
        "joined             {}/{}  ({:.1}%)",
        n_joined,
        outcomes.len(),
        100.0 * n_joined as f64 / outcomes.len() as f64
    );
    println!("failed             {n_failed}");
    println!("join throughput    {:.0} joins/s", n_joined as f64 / secs);
    println!("peak concurrent    {}", stats.peak.load(Ordering::Relaxed));
    println!(
        "bytes received     {:.1} MiB  ({:.1} MiB/s)",
        total_bytes as f64 / (1024.0 * 1024.0),
        total_bytes as f64 / (1024.0 * 1024.0) / secs
    );
    println!("\nlatency to tcp-connect (ms)");
    println!(
        "  p50 {:.1}   p90 {:.1}   p99 {:.1}   max {:.1}",
        pct(&connect_ms, 0.50),
        pct(&connect_ms, 0.90),
        pct(&connect_ms, 0.99),
        connect_ms.last().copied().unwrap_or(0.0)
    );
    println!("latency to login-success (ms)");
    println!(
        "  p50 {:.1}   p90 {:.1}   p99 {:.1}   max {:.1}",
        pct(&login_ms, 0.50),
        pct(&login_ms, 0.90),
        pct(&login_ms, 0.99),
        login_ms.last().copied().unwrap_or(0.0)
    );
    println!("latency to play-ready (ms)");
    println!(
        "  p50 {:.1}   p90 {:.1}   p99 {:.1}   max {:.1}",
        pct(&join_ms, 0.50),
        pct(&join_ms, 0.90),
        pct(&join_ms, 0.99),
        join_ms.last().copied().unwrap_or(0.0)
    );

    if !hist.is_empty() {
        println!("\nerrors by phase");
        let mut rows: Vec<(&str, usize)> = hist.into_iter().collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.1));
        for (phase, count) in rows {
            println!("  {phase:<16} {count}");
        }
    }
    println!("=================================================");
}
