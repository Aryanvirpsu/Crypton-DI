FROM rust:1.76 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/crypton-gateway /usr/local/bin/

EXPOSE 8090
ENTRYPOINT ["/usr/local/bin/crypton-gateway"]
