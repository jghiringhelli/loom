# ── Stage 1: Builder ──────────────────────────────────────────────────────────
# Use the official Rust image pinned to stable for reproducible builds.
FROM rust:1.87-slim AS builder

# Cache-bust argument — change to force a full rebuild.
ARG BUILD_DATE=2026-04-14a

WORKDIR /build

# Install system dependencies needed by rusqlite (bundled) and reqwest (TLS).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the full source tree and build the release binary.
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --bin loom

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
# Minimal Debian image — no Rust toolchain needed at runtime.
FROM debian:bookworm-slim AS runtime

# Install runtime-only dependencies: libssl for reqwest TLS, ca-certificates for
# HTTPS to Claude API and Ollama.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for future hardening reference.
# NOTE: Railway volumes mount as root, so we run as root to allow writes to /data.
RUN useradd --system --create-home --shell /usr/sbin/nologin bioiso

# The SQLite database is stored on a mounted volume.
RUN mkdir -p /data

COPY --from=builder /build/target/release/loom /usr/local/bin/loom
COPY scripts/start-colony.sh /usr/local/bin/start-colony.sh
RUN chmod +x /usr/local/bin/start-colony.sh

# Run as root so the Railway volume mount at /data is writable.
# The container is sandboxed by Railway's platform isolation.
WORKDIR /root

# ── Environment defaults ───────────────────────────────────────────────────────
# All values can be overridden in the Railway service variables UI or .env file.

# Path to the SQLite signal store (mounted volume in production).
ENV DB_PATH=/data/bioiso.db

# Orchestrator tick interval in milliseconds (5 seconds default).
ENV TICK_MS=5000

# Ollama base URL — set to your local/remote Ollama instance if Tier 2 is desired.
# Leave empty to skip Tier 2 and escalate directly to Tier 3 (Claude).
ENV OLLAMA_BASE_URL=""

# Claude API key — required for Tier 3 (Mammal Brain) synthesis.
# Injected as a Railway secret variable; never committed to source.
ENV CLAUDE_API_KEY=""

# Log verbosity: error | warn | info | debug | trace
ENV RUST_LOG=info

# ── Health check ──────────────────────────────────────────────────────────────
# Check that the binary is present and responsive.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD loom runtime status --db ${DB_PATH} || exit 1

# ── Entrypoint ────────────────────────────────────────────────────────────────
# Start the CEMS evolution daemon.  Signal store is opened at DB_PATH.
# The daemon blocks until Ctrl-C or SIGTERM (Railway sends SIGTERM on redeploy).
ENTRYPOINT ["/bin/sh", "/usr/local/bin/start-colony.sh"]
