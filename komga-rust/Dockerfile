FROM node:18-alpine AS webui-build

WORKDIR /webui
COPY komga-webui/package.json komga-webui/package-lock.json ./
RUN npm ci

COPY komga-webui/ ./
RUN npm run build

# Build stage for Rust
FROM rust:latest AS rust-build

WORKDIR /build
COPY komga-rust/Cargo.toml komga-rust/Cargo.lock ./
COPY komga-rust/src ./src
COPY komga-rust/migrations ./migrations

RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy webui build
COPY --from=webui-build /webui/dist /app/webui-dist

# Copy Rust binary
COPY --from=rust-build /build/target/release/komga-rust /app/komga-rust
COPY komga-rust/migrations /app/migrations

# Create non-root user
RUN useradd -m -s /bin/bash appuser && chown -R appuser:appuser /app
USER appuser

EXPOSE 8080

CMD ["/app/komga-rust"]