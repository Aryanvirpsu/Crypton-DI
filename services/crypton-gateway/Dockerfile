# Build context is the REPOSITORY ROOT (the gateway depends on ../../risk-engine):
#   docker build -f services/crypton-gateway/Dockerfile .
FROM rust:1.89-bookworm AS builder
WORKDIR /app

COPY risk-engine ./risk-engine
WORKDIR /app/services/crypton-gateway
# tests-support is a dev-dependency path crate: Cargo must find its manifest even
# for a release build, though it is not compiled into the binary.
COPY services/crypton-gateway/Cargo.toml services/crypton-gateway/Cargo.lock ./
COPY services/crypton-gateway/tests-support ./tests-support
COPY services/crypton-gateway/src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 crypton
COPY --from=builder /app/services/crypton-gateway/target/release/crypton-gateway /usr/local/bin/
USER crypton

# 8090 is the ONE externally exposed port of the whole stack. The internal
# listener (INTERNAL_PORT, default 9090) binds 127.0.0.1 inside the container and
# is deliberately not EXPOSEd or published.
EXPOSE 8090
ENTRYPOINT ["/usr/local/bin/crypton-gateway"]
