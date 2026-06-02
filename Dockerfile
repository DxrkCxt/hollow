# syntax=docker/dockerfile:1
# SPDX-License-Identifier: GPL-3.0-only

# ---------------------------------------------------------------------------
# Builder: compile the `hollow` binary (release) against the workspace.
# BuildKit cache mounts keep the cargo registry and target/ warm across builds.
# ---------------------------------------------------------------------------
FROM rust:slim-bookworm AS builder

WORKDIR /build
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin hollow && \
    cp target/release/hollow /usr/local/bin/hollow

# ---------------------------------------------------------------------------
# Runtime: a minimal Debian image with just the binary and a non-root user.
# settings.yml is written into the working dir (/data) on first start.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Non-root user; /data holds the generated settings.yml and any @-referenced secrets.
RUN useradd --system --uid 10001 --user-group hollow \
    && mkdir -p /data \
    && chown hollow:hollow /data

COPY --from=builder /usr/local/bin/hollow /usr/local/bin/hollow

USER hollow
WORKDIR /data
VOLUME ["/data"]

# Default bind; override per-instance with HOLLOW_BIND / HOLLOW_PORT or CLI flags.
ENV HOLLOW_BIND=0.0.0.0:25565
EXPOSE 25565

# Self-contained probe: connects to the configured port (reads the same env), exit 0/1.
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD ["hollow", "--healthcheck"]

ENTRYPOINT ["hollow"]
