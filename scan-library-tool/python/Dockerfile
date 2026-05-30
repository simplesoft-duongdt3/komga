# ── Rust build stage ──────────────────────────────────────────
FROM rust:1.85-slim AS rust-builder
WORKDIR /build
COPY pdf-hasher/ .
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

# ── Python runtime stage ──────────────────────────────────────
FROM python:3.12-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /build/target/release/pdf-hasher /usr/local/bin/

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY config.py db.py walker.py diff.py api.py applier.py main.py server.py export_json.py perf.py VERSION ./
COPY static/ ./static/

EXPOSE 5050
CMD ["python", "server.py"]
