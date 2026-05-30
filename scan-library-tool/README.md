# Komga Smart Scanner

External diff-based scanner for Komga. Walks the filesystem and reads
PostgreSQL directly to compute what changed, then drives Komga's REST API
— avoiding the full `ScanLibrary` pipeline.

Two implementations are provided:

| | Python | Rust |
|---|---|---|
| Location | [`python/`](python/) | [`rust/`](rust/) |
| Entry point | `python server.py` | `cargo run --release -- serve` |
| CLI mode | `python main.py` | `cargo run --release -- scan` |
| Docker | `python/Dockerfile` | `rust/Dockerfile` |

Both produce identical results — verified by
[compatibility tests](compat-tests/). See
[`compat-tests/COMPARISON.md`](compat-tests/COMPARISON.md) for a detailed
feature-by-feature comparison.

---

## Quick Start

Pick one implementation:

### Python

```bash
cd python
pip install -r requirements.txt
export KOMGA_BASE_URL=http://localhost:25600
# ... set PG_* and KOMGA_PASSWORD ...
python server.py
```

### Rust

```bash
cd rust
cargo run --release -- serve
```

Open http://localhost:5050

---

## Docker

### Build

#### Python image

```bash
# From repo root
docker build -f python/Dockerfile -t smart-scanner-py python/
```

This builds a multi-stage image:
1. Rust stage compiles `pdf-hasher` (the hash cache binary)
2. Python stage installs dependencies and copies the scanner code

#### Rust image

```bash
# From repo root
docker build -f rust/Dockerfile -t smart-scanner-rs rust/
```

This builds a single-stage image with the release binary.

### Run

#### Standalone (Python)

```bash
docker run -d \
  --name smart-scanner \
  -p 5050:5050 \
  -v /path/to/exports:/exports \
  -e KOMGA_BASE_URL=http://host.docker.internal:25600 \
  -e KOMGA_USER=admin@example.com \
  -e KOMGA_PASSWORD=your-password \
  -e PG_HOST=host.docker.internal \
  -e PG_PORT=5432 \
  -e PG_DB=komga \
  -e PG_USER=komga \
  -e PG_PASSWORD=komga123 \
  smart-scanner-py
```

#### Standalone (Rust)

```bash
docker run -d \
  --name smart-scanner \
  -p 5050:5050 \
  -v /path/to/exports:/exports \
  -e KOMGA_BASE_URL=http://host.docker.internal:25600 \
  -e KOMGA_USER=admin@example.com \
  -e KOMGA_PASSWORD=your-password \
  -e PG_HOST=host.docker.internal \
  -e PG_PORT=5432 \
  -e PG_DB=komga \
  -e PG_USER=komga \
  -e PG_PASSWORD=komga123 \
  smart-scanner-rs
```

### Docker Compose

The repo includes a `docker-compose.yml` with a `smart-scanner` service
that runs alongside Komga and PostgreSQL:

```yaml
# Relevant excerpt from docker-compose.yml
services:
  smart-scanner:
    image: scan-library-tool:latest
    container_name: komga-smart-scanner
    depends_on:
      postgres:
        condition: service_healthy
      komga:
        condition: service_started
    ports:
      - "5050:5050"
    environment:
      KOMGA_BASE_URL: http://komga:25600
      KOMGA_USER: admin@gmail.com
      KOMGA_PASSWORD: "your-password"
      PG_HOST: postgres
      PG_PORT: "5432"
      PG_DB: komga
      PG_USER: komga
      PG_PASSWORD: komga123
      DRY_RUN: "false"
      PORT: "5050"
      SCAN_THREADS: "0"
      HASH_CACHE_DIR: /exports
```

To use the Rust image instead, change the `image` field:

```yaml
  smart-scanner:
    image: smart-scanner-rs:latest
```

Then build and start:

```bash
# Build the image first
docker build -f rust/Dockerfile -t smart-scanner-rs:latest rust/

# Start the stack
docker compose up -d
```

### Hash Cache with Docker

The hash cache is stored in a shared volume. Generate it once per library:

```bash
# Python image (includes pdf-hasher binary)
docker compose run --rm smart-scanner pdf-hasher \
  --root /data/manga --cache /exports/hashes-<library-id>.json -j 4

# Rust image (built-in subcommand)
docker compose run --rm smart-scanner komga-smart-scanner hash-cache \
  --root /data/manga --cache /exports/hashes-<library-id>.json -j 4
```

After generating the cache, the scanner picks it up automatically on the next scan.

---

## Compatibility Tests

```bash
# From repo root — runs both and compares
python3 scan-library-tool/compat-tests/compare.py
```

```
Python: 24/24 passed
Rust:   24/24 passed

ALL TESTS MATCH — Python and Rust produce identical results.
```

---

## Environment Variables

Both implementations use the same environment variables:

| Variable | Default | Description |
|---|---|---|
| `KOMGA_BASE_URL` | `http://localhost:8080` | Komga API base URL |
| `KOMGA_USER` | `admin@example.com` | Komga admin username |
| `KOMGA_PASSWORD` | `""` | Komga password |
| `PG_HOST` | `localhost` | PostgreSQL host |
| `PG_PORT` | `5432` | PostgreSQL port |
| `PG_DB` | `komga` | PostgreSQL database |
| `PG_USER` | `komga` | PostgreSQL user |
| `PG_PASSWORD` | `""` | PostgreSQL password |
| `PORT` | `5050` | Web server port |
| `EXPORT_DIR` | `/exports` | JSON + curl export directory |
| `DRY_RUN` | `false` | Preview only (no API calls) |
| `HASH_CACHE_DIR` | `/exports` | Per-library hash cache directory |
| `SCAN_THREADS` | `0` (auto) | FS walker parallelism |
| `HASHER_THREADS` | `4` | Hash cache builder parallelism |
| `MAX_API_RETRIES` | `3` | Komga API retry count |

---

## Architecture

```
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  Scanner      │────▶│  Komga REST   │────▶│  PostgreSQL   │
│  (diff+apply) │     │  (new +       │     │  (read-only)  │
│               │     │   existing)   │     │               │
└───────┬───────┘     └───────────────┘     └───────────────┘
        │
        │ reads
        ▼
┌───────────────┐
│  Filesystem   │
│  (pdf only)   │
└───────────────┘
```
