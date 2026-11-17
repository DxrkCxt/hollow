# Changelog

## Unreleased

### Added
- Minecraft **26.2** (protocol 776) and **26.3** (protocol 777); the supported range is now
  **1.7.2 → 26.3** (26.1.1 and 26.1.2 already share protocol 775 with 26.1).
  - 26.2: Login Success carries the server session id; Join Game gained the `onlineMode` flag.
  - 26.3: new clientbound packet ids in configuration and play; Join Game writes the game
    mode as a VarInt and the previous game mode as an optional VarInt; chunk light masks use
    the byte-array `BitSet` encoding; registry entries may be non-compound NBT
    (`minecraft:block_transformer` entries are lists).
  - Registry codec and tag data for 26.2 and 26.3.

### Fixed
- Player Info (1.19.3+): the game mode is written before the listed flag, in action order.
  Clients saw game mode 1 (creative) whatever the configured mode, and with `gameMode: 0`
  the tab-list entry was also unlisted.

## 1.0.0

Initial **Hollow** release.

### Added
- Async Minecraft *limbo* server on `tokio` (no thread-per-player).
- Multi-version support: protocol **1.7.2 → 26.1**.
- BungeeCord (legacy), BungeeGuard, and Velocity (modern) info forwarding.
- Full MiniMessage text format and drop-in `settings.yml` configuration.
- Console commands: `help`, `conn`, `mem`, `version`, `stop`.
- **Full env/CLI configuration** (itzg-style): every `settings.yml` key is exposed as a
  `HOLLOW_*` environment variable, the common ones as CLI flags, plus a `--set
  key.path=value` escape hatch for anything else. Overrides are layered onto the YAML
  before parsing, so they reuse all of the file's validation. Precedence:
  CLI > environment > `settings.yml` > built-in defaults.
- **`hollow --healthcheck`** — self-contained TCP probe (exit 0 up / 1 down), wired into
  the Docker image as a `HEALTHCHECK`.
- **Docker** image (`Dockerfile`) and **Compose** file (`compose.yml`) with a
  port-range publish for one-command multi-instance scaling.
- CI: cross-platform build/test/clippy matrix and a GHCR image-publish workflow.

### Structure
- Reorganised into a Cargo **workspace** of focused crates:
  `hollow-protocol`, `hollow-text`, `hollow-data`, `hollow-world`,
  `hollow-server`, and the umbrella `hollow` (library + binary).
