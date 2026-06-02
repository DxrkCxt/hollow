# Changelog

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
