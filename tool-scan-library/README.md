# Komga Fast Library Scan Tool

High-performance Python tool for syncing PDF books and JPG thumbnails from the filesystem into Komga's PostgreSQL database. Designed for large libraries (1500+ series, 300k+ books) where Komga's built-in scanner is too slow.

## Features

- **Multi-threaded filesystem scanning** — parallel directory traversal using `ThreadPoolExecutor`
- **Mylar series.json parsing** — extracts title, summary, status, publisher, age rating
- **SIDECAR thumbnail discovery** — matches `poster.jpg`, `cover.jpg` for series and `{book}-0001.jpg` for books
- **PDF analysis** — page count, cropBox dimensions per page, SHA-256 file hash via PyMuPDF
- **Diff-based sync** — compares disk state to DB state, produces minimal INSERT/UPDATE/DELETE
- **Two SQL export modes** — generate scan SQL and analyze SQL separately for safe production deployment
- **Batch commit** — configurable batch sizes for 300k+ book datasets

## Requirements

- Python 3.11+
- PostgreSQL (read-only access for diff, write access for sync)
- PyMuPDF (for PDF analysis phase only)

## Quick Start

```bash
cd tool-scan-library
pip install -r requirements.txt

# Set required environment variables
export KOMGA_DB_USER=ai_readonly
export KOMGA_DB_PASS=ai_readonly_pass
export KOMGA_DB_WRITE_USER=komga_admin
export KOMGA_DB_WRITE_PASS=secret
export KOMGA_LIBRARY_ROOT=/data/library
export KOMGA_LIBRARY_ID=0Q3CKC76902B7

# Dry-run: scan only, no DB writes
python main.py --dry-run

# Full sync: scan + diff + apply
python main.py

# Full pipeline: scan + sync + analyze (page count, dimensions, hash)
python main.py --analyze
```

## Configuration

All settings can be set via environment variables or command-line flags. CLI flags override env vars.

| Env Variable | CLI Flag | Default | Description |
|---|---|---|---|
| `KOMGA_DB_HOST` | `--db-host` | `192.168.1.169` | PostgreSQL host |
| `KOMGA_DB_PORT` | `--db-port` | `5433` | PostgreSQL port |
| `KOMGA_DB_NAME` | `--db-name` | `komga` | Database name |
| `KOMGA_DB_USER` | `--db-user` | `ai_readonly` | Read-only DB user |
| `KOMGA_DB_PASS` | `--db-pass` | — | Read-only password |
| `KOMGA_DB_WRITE_USER` | `--write-user` | — | Write-capable DB user (falls back to read-only) |
| `KOMGA_DB_WRITE_PASS` | `--write-pass` | — | Write password |
| `KOMGA_DB_MIN_CONN` | — | `2` | Min connection pool size |
| `KOMGA_DB_MAX_CONN` | — | `10` | Max connection pool size |
| `KOMGA_LIBRARY_ID` | `--library-id` | `0Q3CKC76902B7` | Komga library UUID |
| `KOMGA_LIBRARY_ROOT` | `--library-root` | — | Library root path (same as mounted in Komga) |
| `KOMGA_SCAN_WORKERS` | `--workers` | `cpu_count × 2` (max 32) | Scanner/analyzer thread count |
| `KOMGA_BATCH_SIZE` | `--batch-size` | `5000` | DB commit batch size |

## CLI Reference

### Scan & Sync

```bash
# Dry-run (scan only, print summary)
python main.py --dry-run

# Sync with defaults
python main.py

# Sync with explicit path
python main.py --library-root /data/library \
  --write-user admin --write-pass secret

# Export scan SQL to file (no direct DB writes)
python main.py --export-sql scan.sql

# Sync with custom thread count and batch size
python main.py --workers 16 --batch-size 5000
```

### Analysis (PDF page count, dimensions, SHA-256)

```bash
# Full pipeline: scan + sync + analyze
python main.py --analyze

# Analyze at most 100 books
python main.py --analyze --analyze-limit 100

# Analyze without hashing (fast — ~3 min for 300k books)
python main.py --analyze --no-hash

# Analyze page count only, skip dimensions
python main.py --analyze --no-dimensions

# Generate analyze SQL for later deployment
python main.py --analyze --analyze-sql analyze.sql

# Full pipeline with SQL export for both phases
python main.py --analyze --export-sql scan.sql --analyze-sql analyze.sql
```

## Docker Deployment

The tool can run inside the same Docker environment as Komga with identical volume mounts, eliminating the need for path mapping.

```bash
# Build the scanner image
cd tool-scan-library
docker build -t komga-scanner:latest .

# Run as a one-off container alongside Komga
docker compose run --rm scanner --dry-run

# Full sync + analyze from within Docker
docker compose run --rm scanner --analyze --no-hash

# Export SQL for safe production deployment
docker compose run --rm scanner --export-sql scan.sql
docker compose run --rm scanner --analyze --analyze-sql analyze.sql
```

When running inside Docker, the `--library-root` path is the same as what Komga stores in the database — no conversion needed. Set `KOMGA_LIBRARY_ROOT` to the container path (e.g., `/data/data-books-audiobooks/Manga_Ebook/Manhwa`).

## Two-Phase Workflow

### Phase 1: Scan

1. Walks the filesystem root, discovering series directories and PDF books
2. Parses `series.json` for metadata
3. Matches JPG thumbnails to series and books
4. Compares disk state against database state (diff)
5. Computes minimal changes: new series/books, deleted series/books, metadata updates
6. Writes SQL or applies changes directly

### Phase 2: Analyze (optional, `--analyze`)

1. Queries books with `MEDIA.STATUS = 'UNKNOWN'` (set by scan phase)
2. Opens each PDF with PyMuPDF to extract page count and cropBox dimensions
3. Computes SHA-256 file hash (skippable with `--no-hash`)
4. Updates `MEDIA` (status → READY, page count, media type)
5. Inserts `MEDIA_PAGE` rows (per-page dimensions, ON CONFLICT upsert)
6. Updates `BOOK.FILE_HASH`

## SQL Export Workflow

For safe production deployment, generate SQL files and apply them separately:

```bash
# Step 1: Generate scan SQL
python main.py --export-sql scan.sql

# Step 2: Apply scan SQL (creates books with STATUS='UNKNOWN')
psql -h db -U komga -d komga -f scan.sql

# Step 3: Generate analyze SQL (reads UNKNOWN books, generates analysis)
python main.py --analyze --analyze-sql analyze.sql

# Step 4: Apply analyze SQL (updates STATUS='READY', inserts page data)
psql -h db -U komga -d komga -f analyze.sql
```

**Why two separate files?** The analyze SQL references book IDs that the scan SQL creates. They must be applied in order: first scan.sql creates the rows, then analyze.sql updates them.

## Performance

| Operation | Per-book time | 300k books (16 threads) |
|---|---|---|
| Page count only | ~5 ms | ~1.6 min |
| Page count + dimensions | ~10 ms | ~3 min |
| SHA-256 hash | ~100 ms (30 MB file) | ~30 min |
| Full analysis | ~110 ms | ~35 min |

**Hashing is the bottleneck.** Use `--no-hash` for speed-critical pipelines (~3 min vs ~35 min).

## Project Structure

```
tool-scan-library/
├── main.py            CLI entry point, flow orchestration
├── scanner.py         Filesystem scanner, series.json parser, thumbnail matcher
├── syncer.py          Diff engine — compares disk vs DB, produces DiffResult
├── db.py              PostgreSQL operations — read queries and write batch methods
├── analyzer.py        PDF analysis via PyMuPDF — page count, dimensions, hash
├── sql_exporter.py    SQL generation — scan diff SQL and analyze SQL
├── config.py          Dataclass configuration with env-var loading
├── id_generator.py    UUID v4 ID generation
└── requirements.txt   Python dependencies
```

## Notes

- Only PDF books (`.pdf`) are supported. EPUB and CBZ are not scanned by this tool.
- SIDECAR JPG thumbnails from the filesystem are the only thumbnail type created. GENERATED thumbnails are never produced — Komga's frontend uses SIDECAR first, then GENERATED as fallback.
- When running inside the same Docker environment as Komga, library paths are identical — no path conversion needed. The tool uses the filesystem path directly, which matches what Komga stores in the DB.
- MEDIA_PAGE dimensions use `cropBox` (not `mediaBox`) to match Komga's PDF analyzer behavior.


cd /Users/teamcumahay/Documents/GitHub/komga/tool-scan-library && python3 main.py \
  --library-root "/Users/teamcumahay/Downloads/ThienThaiTruyen1" \
  --export-sql /tmp/test-scan2.sql \
  --analyze --analyze-sql /tmp/test-analyze.sql \
  --analyze-limit 3 \
  --workers 2 \
  --verbose 2>&1