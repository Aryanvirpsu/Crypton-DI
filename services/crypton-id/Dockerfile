# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:1.89-bookworm AS builder

WORKDIR /app

# Install build-time dependencies for the openssl crate
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation before copying real source.
# Trick: compile a dummy main.rs with the real Cargo manifests so the
# dependency layer is cached even when src/ changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -f target/release/crypton-identity target/release/deps/crypton_identity*

# Copy real source and compile
COPY src ./src
COPY migrations ./migrations
COPY test-webauthn.html ./test-webauthn.html

RUN cargo build --release

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Binary
COPY --from=builder /app/target/release/crypton-identity /usr/local/bin/crypton-identity

# sqlx migrate! embeds migrations at compile time via include_str! — no runtime copy needed.
# But keep a copy for debugging / manual migration runs.
COPY --from=builder /app/migrations /migrations

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/crypton-identity"]
