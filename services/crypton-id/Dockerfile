FROM rust:1.88 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY test-webauthn.html ./test-webauthn.html

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/crypton-identity /usr/local/bin/crypton-identity
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/test-webauthn.html ./test-webauthn.html

EXPOSE 8080
CMD ["crypton-identity"]
