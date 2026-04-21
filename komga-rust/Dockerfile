FROM rust:latest AS build

WORKDIR /build

COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /build/target/release/komga-rust /app/komga-rust
COPY migrations /app/migrations

EXPOSE 8080

CMD ["/app/komga-rust"]