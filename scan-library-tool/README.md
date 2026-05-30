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

## Compatibility Tests

```bash
# From repo root — runs both and compares
python3 compat-tests/compare.py
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
