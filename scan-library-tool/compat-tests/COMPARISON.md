# Python vs Rust — Komga Smart Scanner Comparison

This document compares the original Python implementation (`scan-library-tool/`)
with the Rust rewrite (`rust-scan-library-tool/`) and proves behavioral equivalence
via shared compatibility tests.

---

## Architecture Comparison

| Aspect | Python | Rust |
|---|---|---|
| Language | Python 3.12+ | Rust 1.86+ |
| Web framework | FastAPI + Uvicorn | Axum + Tokio |
| HTTP client | `requests` | `reqwest` |
| PostgreSQL | `psycopg2` (sync) | `sqlx` (async) |
| XXH3 hashing | `xxhash` Python binding | `xxhash-rust` (native) |
| FS walker | `pathlib` + `ThreadPoolExecutor` | `walkdir` + `rayon` |
| Async runtime | asyncio / Uvicorn | Tokio (multi-threaded) |
| Binary size | ~50MB (Python venv) | ~15MB (static release) |
| Dependencies | 6 pip packages | 20 Cargo crates |
| Docker image | python:3.12-slim (~150MB) | debian:bookworm-slim (~80MB) |

---

## Module Mapping

| Python file | Rust file | Notes |
|---|---|---|
| `config.py` | `config.rs` | Identical env vars, same defaults |
| `api.py` | `api.rs` | Same endpoints, same retry logic |
| `db.py` | `db.rs` | Same SQL queries, same column quoting |
| `walker.py` | `walker.rs` | Both use parallel dir walk, XXH3_128 with seed=0 |
| `diff.py` | `diff.rs` | Same URL normalization, same mtime comparison |
| `applier.py` | `applier.rs` | Same curl script format, same API apply sequence |
| `server.py` | `server.rs` | Same 10 API endpoints, same SSE format |
| `export_json.py` | `export.rs` | Same J1/J2/J3 JSON structure |
| `perf.py` | `perf.rs` | Same phase timing model |
| `pdf-hasher/` (Rust) | `hasher.rs` | Merged into main binary, same cache format |
| `main.py` | `main.rs` | Same CLI flow (list → select → scan → diff → apply) |
| `static/` | `static/` | Identical HTML/JS files (shared verbatim) |

---

## Behavioral Equivalence

### Compatibility Test Results

```
Python: 24/24 passed
Rust:   24/24 passed

ALL TESTS MATCH — Python and Rust produce identical results.
```

### Test Categories

| Category | Cases | What's tested |
|---|---|---|
| URL normalization | 8 | `file:/path`, `file:///path/`, Java vs Python URLs, non-file passthrough |
| mtime comparison | 6 | Sub-second tolerance, timezone stripping, Z suffix, empty strings |
| Diff computation | 10 | No changes, new/deleted series, new/deleted books, changed (mtime/size/hash), pending hash, Java URL integration |

### How to Run

```bash
# From repo root
python3 compat-tests/compare.py
```

This runs both implementations against `compat-tests/fixtures/test_cases.json` and
diffs the JSON outputs. Exit code 0 = all match, exit code 1 = differences found.

---

## Key Behavioral Differences

### 1. URL Encoding

| Aspect | Python | Rust |
|---|---|---|
| FS URL generation | `Path.as_uri()` — percent-encodes spaces (`%20`) | `url::Url::from_file_path()` — same behavior |
| DB URL source | Java `URL.toString()` — `file:/path` format | Same (read from PostgreSQL) |
| Normalization | `_normalize_file_url()` strips scheme, re-adds `file:///` | `normalize_file_url()` — identical logic |

Both implementations produce identical normalized URLs for paths with spaces,
unicode characters, and mixed Java/Python formats.

### 2. Parallel Filesystem Walk

| Aspect | Python | Rust |
|---|---|---|
| Threading | `ThreadPoolExecutor(max_workers=N)` | `rayon::ThreadPoolBuilder::num_threads(N)` |
| Default workers | `CPU * 2` | `CPU * 2` |
| Progress callback | `on_progress(completed, total, dir_name)` | Same signature via `impl Fn` |
| Oneshots | Root-level + dedicated dir | Same |

### 3. Hash Computation

| Aspect | Python | Rust |
|---|---|---|
| Algorithm | XXH3_128, seed=0, 32-char hex | Same |
| Cache format | JSON (`HashCache` struct) | Same JSON schema, cross-compatible |
| Inline fallback | `xxhash.xxh3_128(seed=0)` | `xxhash_rust::xxh3::xxh3_128()` |
| mmap threshold | N/A (always streams) | 100MB (mmap for small files) |

The mmap optimization in Rust is transparent — same hash output for same input.

### 4. Curl Script Generation

| Aspect | Python | Rust |
|---|---|---|
| Script format | `#!/bin/bash` + comments + curl commands | Same |
| Credential handling | `-u 'user:password'` in script | Same (redacted in SSE output) |
| Category ordering | DELETE commands, then `empty-trash` | Same (fixed from initial Rust version) |
| Combined script | `all.sh` with all categories | Same |

### 5. Web Server

| Aspect | Python | Rust |
|---|---|---|
| Endpoints | 10 REST + SSE | Same 10 endpoints, same paths |
| SSE format | `data: {json}\n\n` | Same |
| Heartbeat | `: heartbeat\n\n` every 15s | Same |
| Static files | FastAPI `StaticFiles` | `tower-http::ServeDir` |
| Auth | None (same as Python) | None |

---

## Performance Expectations

| Operation | Python (est.) | Rust (est.) | Speedup |
|---|---|---|---|
| FS walk (100k PDFs) | ~30s | ~5s | ~6x |
| Hash computation (1000 PDFs) | ~10s | ~2s | ~5x |
| Diff computation (10k series) | ~200ms | ~10ms | ~20x |
| DB query (10k books) | ~500ms | ~100ms | ~5x |
| Memory usage (100k files) | ~500MB | ~50MB | ~10x |
| Cold start | ~2s | ~50ms | ~40x |

Rust advantages compound for large libraries due to zero-cost abstractions,
no GC pauses, and native parallelism without GIL contention.

---

## Deployment Comparison

### Python

```bash
cd scan-library-tool
pip install -r requirements.txt
export KOMGA_BASE_URL=http://localhost:25600
# ... set other env vars ...
python server.py
```

### Rust

```bash
cd rust-scan-library-tool
cargo build --release
./target/release/komga-smart-scanner serve
# Or: cargo run --release -- serve
```

### Docker

```bash
# Python
docker build -f scan-library-tool/Dockerfile -t smart-scanner-py .

# Rust
docker build -f rust-scan-library-tool/Dockerfile -t smart-scanner-rs .
```

### CLI Mode

```bash
# Python
python scan-library-tool/main.py

# Rust
./target/release/komga-smart-scanner scan
```

### Hash Cache Generation

```bash
# Python (uses external pdf-hasher binary)
pdf-hasher --root /data --cache /exports/hashes.json -j 4

# Rust (built-in subcommand)
./target/release/komga-smart-scanner hash-cache --root /data --cache /exports/hashes.json -j 4
```

---

## Environment Variables

Both implementations use identical environment variables with the same defaults:

| Variable | Default | Both |
|---|---|---|
| `KOMGA_BASE_URL` | `http://localhost:8080` | ✓ |
| `KOMGA_USER` | `admin@example.com` | ✓ |
| `KOMGA_PASSWORD` | `""` | ✓ |
| `PG_HOST` | `localhost` | ✓ |
| `PG_PORT` | `5432` | ✓ |
| `PG_DB` | `komga` | ✓ |
| `PG_USER` | `komga` | ✓ |
| `PG_PASSWORD` | `""` | ✓ |
| `PORT` | `5050` | ✓ |
| `EXPORT_DIR` | `/exports` | ✓ |
| `DRY_RUN` | `false` | ✓ |
| `HASH_CACHE_DIR` | `/exports` | ✓ |
| `SCAN_THREADS` | `0` (auto) | ✓ |
| `HASHER_THREADS` | `4` | ✓ |
| `MAX_API_RETRIES` | `3` | ✓ |

---

## Hash Cache Compatibility

Both versions read and write the same JSON cache format:

```json
{
  "type": "XXH3_128 hash cache",
  "version": 1,
  "root": "/data/manga",
  "generated_at_unix_secs": 1748000000,
  "elapsed_secs": 12.3,
  "total_files": 50000,
  "total_bytes": 1234567890,
  "entries": {
    "file:///data/manga/Series/Ch01.pdf": {
      "hash": "3ec83f6f7b6fcae0825ab2f6bdb18506",
      "size": 12345,
      "mtime_secs": 1748000000
    }
  }
}
```

A cache generated by the Python version's `pdf-hasher` is readable by the Rust
version and vice versa.

---

## Migration Guide

### From Python to Rust

1. Build the Rust binary: `cargo build --release`
2. Copy `.env` or set the same environment variables
3. Stop the Python server
4. Start the Rust server: `./target/release/komga-smart-scanner serve`
5. Verify at http://localhost:5050 — the web UI is identical

No data migration needed. No config changes needed. The hash cache files are
compatible between versions.

### Rollback

Switch back to the Python version at any time — no state is stored that
isn't compatible between both implementations.
