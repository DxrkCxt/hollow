# Hollow

A lightweight Minecraft *limbo* server written in Rust. It holds players in an empty
world while the main server is unavailable, sending the minimum number of packets needed
to keep a connection alive.

Author: **DxrkCxt**.

## Features

- High-performance async networking on `tokio` (no thread-per-player).
- Multi-version support: protocol **1.7.2 → 26.1**.
- BungeeCord (legacy), BungeeGuard, and Velocity (modern) info forwarding.
- Full MiniMessage text format (gradient, rainbow, colours, decorations, click/hover, …).
- Drop-in `settings.yml`, plus CLI flags and `HOLLOW_*` env vars for per-instance config.
- Console commands: `help`, `conn`, `mem`, `version`, `stop`.
- First-class Docker / Compose deployment with one-command multi-instance scaling.

## Workspace layout

Hollow is a Cargo workspace of focused crates (acyclic dependency graph, bottom-up):

| Crate | Responsibility |
|-------|----------------|
| `hollow-protocol` | `ByteMessage`, protocol versions, packet registry, snapshots, NBT/UUID/JSON helpers |
| `hollow-text` | component model, colours, legacy codes, gson, MiniMessage parser/serializer |
| `hollow-data` | shared config data types (namespaced key, boss bar, title, ping, forwarding, transport) |
| `hollow-world` | dimension types and the embedded per-version codec/tags registry |
| `hollow-server` | packets, connection pipeline, accept loop, configuration, commands, forwarding |
| `hollow` | umbrella crate (re-exports) **and** the `hollow` binary |

## Build

Requires a Rust toolchain new enough for edition 2024 (MSRV **1.96**).

```sh
cargo build --release --bin hollow
```

The binary is produced at `target/release/hollow` (`.exe` on Windows).

## Run

```sh
cargo run --release --bin hollow
```

On first start a default `settings.yml` is written next to the binary; edit it to
configure MOTD, forwarding, etc.

## Configuration

Hollow can be configured entirely without touching `settings.yml` — like
`itzg/minecraft-server`, **every** setting is exposed as a `HOLLOW_*` environment
variable, with the common ones also available as CLI flags. Precedence, highest first:

1. **CLI flags** — see `hollow --help`
2. **Environment** — `HOLLOW_*`
3. **`settings.yml`**
4. **Built-in defaults**

Env vars cover the whole file. A selection:

| Variable | settings.yml key | Notes |
|----------|------------------|-------|
| `HOLLOW_BIND` | `bind.ip` + `bind.port` | `host:port`; `HOLLOW_HOST` / `HOLLOW_PORT` override either half (`*` = all interfaces) |
| `HOLLOW_MAX_PLAYERS` | `maxPlayers` | |
| `HOLLOW_MOTD` | `ping.description` | MiniMessage |
| `HOLLOW_VERSION_NAME`, `HOLLOW_PROTOCOL` | `ping.*` | |
| `HOLLOW_DIMENSION`, `HOLLOW_GAMEMODE`, `HOLLOW_SECURE_PROFILE` | top-level | |
| `HOLLOW_BRAND`, `HOLLOW_JOIN_MESSAGE`, `HOLLOW_PLAYER_LIST_USERNAME` | sections | setting the text auto-enables the section |
| `HOLLOW_HEADER`, `HOLLOW_FOOTER` | `headerAndFooter.*` | |
| `HOLLOW_BOSSBAR_TEXT/COLOR/DIVISION/HEALTH` | `bossBar.*` | |
| `HOLLOW_TITLE`, `HOLLOW_SUBTITLE`, `HOLLOW_TITLE_FADE_IN/STAY/FADE_OUT` | `title.*` | |
| `HOLLOW_FORWARDING[_TYPE]`, `HOLLOW_FORWARDING_SECRET`, `HOLLOW_FORWARDING_TOKENS` | `infoForwarding.*` | secret accepts `@file`; tokens are comma/newline separated |
| `HOLLOW_READ_TIMEOUT`, `HOLLOW_DEBUG_LEVEL`, `HOLLOW_LOG_PLAYERS_IP` | top-level | |
| `HOLLOW_TRANSPORT_TYPE`, `HOLLOW_WORKER_THREADS` | `netty.*` | |
| `HOLLOW_TRAFFIC_ENABLE`, `HOLLOW_MAX_PACKET_SIZE`, `HOLLOW_TRAFFIC_INTERVAL`, `HOLLOW_MAX_PACKET_RATE`, `HOLLOW_MAX_PACKET_BYTES_RATE` | `traffic.*` | |

Anything not listed (or any future key) is reachable via the `--set` escape hatch:

```sh
hollow --bind 0.0.0.0:25565 --max-players 0 --motd "<rainbow>lobby"   # flags
HOLLOW_PORT=25566 HOLLOW_FORWARDING=MODERN HOLLOW_FORWARDING_SECRET=@/run/secrets/velocity hollow
hollow --set traffic.maxPacketRate=800 --set bossBar.color=PURPLE     # any key, dotted
```

### Health check

`hollow --healthcheck` connects to the configured port and exits `0` (up) or `1` (down) —
self-contained, no extra tooling. The Docker image wires this into a `HEALTHCHECK`
automatically.

## Docker

Build and run a single instance:

```sh
docker compose up --build
```

Pull the prebuilt image from GHCR instead of building:

```sh
docker pull ghcr.io/dxrkcxt/hollow:latest
docker run --rm -p 25565:25565 -e HOLLOW_BIND=0.0.0.0:25565 ghcr.io/dxrkcxt/hollow:latest
```

### Deploy many instances

`compose.yml` publishes a host **port range**, so a single command spins up many
independently-addressable instances (host ports 25565, 25566, …):

```sh
docker compose up -d --scale hollow=10
```

Each replica is configured through the same `HOLLOW_*` env vars; give an
individually-tuned instance its own service block (see the commented example in
`compose.yml`).

## Benchmark

Load test of **10,000 players** performing the full join sequence (handshake → login →
configuration → play) and then held in *Play* for 60 s echoing keep-alives, using the
bundled `bench` example. The numbers below are a single real run, not estimates.

**Test machine:** AMD Ryzen 9 7950X3D (16C/32T) · 64 GB RAM · Windows 11 · loopback
(client and server on the same host) · `--release` build, 32 worker threads, traffic
limiter on, per-connection logging off (`HOLLOW_DEBUG_LEVEL=1`).

| Metric | Result |
|--------|--------|
| Players joined | **10,000 / 10,000** (100%), 0 failed |
| Peak concurrent | 10,000 |
| Time to join all 10k | **~2.5 s** (≈ **4,000 joins/s** avg; ~5,000 in the first 0.5 s) |
| Latency to *play-ready* | p50 **21 ms** · p90 537 ms · p99 1565 ms · max 2068 ms |
| Latency to tcp-connect | p50 0.7 ms · p90 506 ms · p99 1542 ms |
| **Server CPU** holding 10k | **≈ 1% of 32 threads (~0.3 core)** |
| **Server RAM** holding 10k | **≈ 200 MB RSS** (idle 28 MB → **~18 KB / player**) |
| Sustained I/O | 1058 MiB over 60 s (~17 MiB/s, keep-alive echo) |

### Cost per player

Derived from the 10k run (steady state minus the idle baseline, divided by 10,000):

| Resource | Per connected player | Notes |
|----------|----------------------|-------|
| **RAM** | **≈ 18 KB** | (200 MB − 28 MB idle) / 10,000. Connection state + a 4 KB read buffer + the outbound queue, amortised. |
| **CPU** | **≈ nothing** | 10k players cost ~0.3 of one core *total* → ~0.00003 core each. A held player is one keep-alive packet every 5 s. |
| **Network** | **≈ 100 KB once, then ~0** | The join sends registry/codec + chunks (~100 KB, version-dependent). After that it's just the 5 s keep-alive (a few bytes/s). |

On top of that there's a **fixed ~28 MB baseline** per process, independent of player count.

So a connected limbo player is **~18 KB of RAM and effectively no CPU**. RAM is the
binding resource: ~28 MB base + 18 KB × players (e.g. ~1 GB for ~55k, ~2 GB for ~110k).
The join burst is the only real work; holding players afterwards is almost free. On a
single host the practical test ceiling is the OS ephemeral-port range and the load
generator's own cost, not the server.

Reproduce (start the server with `infoForwarding: NONE` and `maxPlayers: 0`, then):

```sh
cargo run --release --example bench -- --addr 127.0.0.1:25565 --players 10000 --concurrency 1000 --hold 60
```

The example reports join throughput, connect/login/play-ready latency percentiles, peak
concurrency, bytes received, and a per-phase error breakdown.

## Continuous integration

- [`build.yml`](.github/workflows/build.yml) — clippy (`-D warnings`), tests, and builds
  for Windows x64, Linux x64, and macOS (Apple Silicon) on every push (CI artifacts).
- [`release.yml`](.github/workflows/release.yml) — on a `v*` tag, builds the binaries and
  publishes a **GitHub Release** with them attached (see below).
- [`docker-publish.yml`](.github/workflows/docker-publish.yml) — builds and pushes the
  runtime image to `ghcr.io/<owner>/hollow` on pushes to `main` and on `v*` tags.

## Cutting a release

Releases are tag-driven. Pushing a `v*` tag builds the binaries and publishes a GitHub
Release (and a matching `ghcr.io/<owner>/hollow:<version>` image) automatically:

```sh
# 1. set the version in the root Cargo.toml ([workspace.package] version), commit it
# 2. tag and push the tag
git tag v1.0.0
git push origin v1.0.0
```

`release.yml` then builds `--release` for Windows x64, Linux x64, and macOS arm64, names
each binary `hollow-v1.0.0-<platform>[.exe]` (e.g. `hollow-v1.0.0-windows-x64.exe`,
`hollow-v1.0.0-macos-arm64`), and uploads all three plus a `SHA256SUMS.txt` to the
release, with auto-generated notes.

To do it by hand instead (e.g. a local build), use the GitHub CLI:

```sh
cargo build --release --bin hollow
gh release create v1.0.0 target/release/hollow --generate-notes
```

## License

Distributed under the **GNU General Public License v3.0** (see [`LICENSE`](LICENSE)).
This is an independent Rust implementation derived from a GPLv3-licensed Minecraft limbo
server; it therefore remains under the GPLv3.
